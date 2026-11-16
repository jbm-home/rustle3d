//! Attached mode (docs/specs/09-mcp-connector.md): the desktop app serves
//! its open project to one agent at a time, so the user sees every change
//! live and can undo it. The app listens on a local TCP port (loopback
//! only) and writes the port and a secret token to `agent.json` in its
//! support folder; `r3d mcp --attach` reads that file, connects, sends the
//! token and relays the agent's stdio. Nothing listens until the user
//! allows agents in the app, and they can disconnect one at any time.

use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rmcp::ServiceExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

use crate::{ServeError, Server};

/// The file the app writes in its support folder while it accepts agents.
pub const ENDPOINT_FILE: &str = "agent.json";
/// The first line a client sends: this word, a space, the token.
const HELLO: &str = "R3D-ATTACH";
/// How long a client has to say hello.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// Where and how to reach the app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub port: u16,
    pub token: String,
    /// The app's process, for information.
    pub pid: u32,
    /// The open project's name, for information.
    pub project: String,
}

/// What the app shows about agents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentState {
    /// Listening; no agent yet.
    Waiting,
    /// An agent is connected.
    Connected,
}

struct Shared {
    connected: AtomicBool,
    disconnect: Notify,
    stop: Notify,
}

/// The app's side: accepts agents until dropped.
pub struct Host {
    shared: Arc<Shared>,
    thread: Option<std::thread::JoinHandle<()>>,
    file: PathBuf,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host").field("file", &self.file).finish()
    }
}

/// A token nobody else can guess: the process's random hashing keys
/// (seeded by the OS) over the time and the process id.
fn new_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    (0..2)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(now);
            h.write_u32(std::process::id());
            h.write_u8(i);
            format!("{:016x}", h.finish())
        })
        .collect()
}

fn io_error(e: impl std::fmt::Display) -> ServeError {
    ServeError::Transport(e.to_string())
}

impl Host {
    /// Listen for agents on `server` (which runs their calls on the app's
    /// project), and write the endpoint into `dir`.
    pub fn start(server: Server, dir: &Path, project: &str) -> Result<Self, ServeError> {
        let shared = Arc::new(Shared {
            connected: AtomicBool::new(false),
            disconnect: Notify::new(),
            stop: Notify::new(),
        });
        let token = new_token();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = {
            let (shared, token) = (shared.clone(), token.clone());
            std::thread::Builder::new()
                .name("r3d-agent".into())
                .spawn(move || {
                    let runtime = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(r) => r,
                        Err(e) => {
                            let _ = ready_tx.send(Err(io_error(e)));
                            return;
                        }
                    };
                    runtime.block_on(accept(server, shared, token, ready_tx));
                })
                .map_err(ServeError::Runtime)?
        };
        let port = ready_rx
            .recv()
            .map_err(|_| io_error("the agent listener stopped"))??;
        let file = dir.join(ENDPOINT_FILE);
        let endpoint = Endpoint {
            port,
            token,
            pid: std::process::id(),
            project: project.to_owned(),
        };
        let host = Self {
            shared,
            thread: Some(thread),
            file,
        };
        std::fs::create_dir_all(dir).map_err(io_error)?;
        let json = serde_json::to_vec_pretty(&endpoint).map_err(io_error)?;
        std::fs::write(&host.file, json).map_err(io_error)?;
        Ok(host)
    }

    pub fn state(&self) -> AgentState {
        if self.shared.connected.load(Ordering::SeqCst) {
            AgentState::Connected
        } else {
            AgentState::Waiting
        }
    }

    /// Close the connected agent's session (others may connect again).
    pub fn disconnect(&self) {
        self.shared.disconnect.notify_waiters();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
        self.shared.stop.notify_one();
        self.shared.disconnect.notify_waiters();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Accept agents one after the other until stopped.
async fn accept(
    server: Server,
    shared: Arc<Shared>,
    token: String,
    ready: std::sync::mpsc::Sender<Result<u16, ServeError>>,
) {
    let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
        Ok(l) => l,
        Err(e) => {
            let _ = ready.send(Err(io_error(e)));
            return;
        }
    };
    match listener.local_addr() {
        Ok(addr) => {
            let _ = ready.send(Ok(addr.port()));
        }
        Err(e) => {
            let _ = ready.send(Err(io_error(e)));
            return;
        }
    }
    loop {
        let stream = tokio::select! {
            _ = shared.stop.notified() => return,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(_) => continue,
            },
        };
        tokio::select! {
            _ = shared.stop.notified() => return,
            _ = session(&server, &shared, &token, stream) => {}
        }
    }
}

/// One agent: check its token, then serve it until it leaves or the user
/// disconnects it.
async fn session(server: &Server, shared: &Shared, token: &str, stream: TcpStream) {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut hello = String::new();
    let said = tokio::time::timeout(HELLO_TIMEOUT, reader.read_line(&mut hello)).await;
    let expected = format!("{HELLO} {token}");
    if !matches!(said, Ok(Ok(_))) || hello.trim_end() != expected {
        let _ = write.write_all(b"denied\n").await;
        return;
    }
    let Ok(service) = server.clone().serve((reader, write)).await else {
        return;
    };
    shared.connected.store(true, Ordering::SeqCst);
    tokio::select! {
        _ = shared.disconnect.notified() => {}
        _ = service.waiting() => {}
    }
    shared.connected.store(false, Ordering::SeqCst);
}

/// Where the app writes its endpoint, by platform (its support folder).
pub fn default_endpoint_files() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        if let Some(home) = &home {
            // Sandboxed builds live in their container.
            found.push(home.join(
                "Library/Containers/dev.rustle3d.rustle3d/Data/Library/Application Support/dev.rustle3d.rustle3d",
            ));
            found.push(home.join("Library/Application Support/dev.rustle3d.rustle3d"));
        }
    } else if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            found.push(PathBuf::from(appdata).join("dev.rustle3d").join("rustle3d"));
        }
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
        if let Some(data) = data {
            found.push(data.join("dev.rustle3d.rustle3d"));
        }
    }
    found.into_iter().map(|d| d.join(ENDPOINT_FILE)).collect()
}

/// The agent's side: connect to the app described in `file` (or the
/// default place) and relay stdin/stdout to it until either side closes.
pub fn relay(file: Option<&Path>) -> Result<(), ServeError> {
    let candidates = match file {
        Some(f) => vec![f.to_path_buf()],
        None => default_endpoint_files(),
    };
    let endpoint: Endpoint = candidates
        .iter()
        .find_map(|f| serde_json::from_slice(&std::fs::read(f).ok()?).ok())
        .ok_or_else(|| ServeError::NotAttached {
            looked: candidates.clone(),
        })?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(ServeError::Runtime)?;
    let relayed = runtime.block_on(async {
        let stream = TcpStream::connect(("127.0.0.1", endpoint.port))
            .await
            .map_err(|_| ServeError::NotAttached {
                looked: candidates.clone(),
            })?;
        let (mut from_app, mut to_app) = stream.into_split();
        to_app
            .write_all(format!("{HELLO} {}\n", endpoint.token).as_bytes())
            .await
            .map_err(io_error)?;
        let (mut stdin, mut stdout) = (tokio::io::stdin(), tokio::io::stdout());
        // Until the app closes the connection (or the agent its input).
        tokio::select! {
            r = pass(&mut from_app, &mut stdout) => r,
            r = pass(&mut stdin, &mut to_app) => r,
        }
    });
    // Reading stdin blocks a thread until the agent closes it, which it may
    // never do once the app is gone: do not wait for it.
    runtime.shutdown_background();
    relayed
}

/// Pass bytes from `from` to `to` as they come, flushing each time
/// (`tokio::io::copy` would keep them in stdout's buffer until the end).
async fn pass(
    from: &mut (impl AsyncRead + Unpin),
    to: &mut (impl AsyncWrite + Unpin),
) -> Result<(), ServeError> {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = from.read(&mut buf).await.map_err(io_error)?;
        if n == 0 {
            return Ok(());
        }
        to.write_all(buf.get(..n).unwrap_or_default())
            .await
            .map_err(io_error)?;
        to.flush().await.map_err(io_error)?;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn tokens_differ_and_are_long() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }
}
