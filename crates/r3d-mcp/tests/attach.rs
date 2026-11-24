#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Attached mode end to end: an agent connects to the app's endpoint with
//! its token, speaks MCP, and its changes reach the shared project; a
//! second agent can connect alongside it.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use r3d_commands::Workspace;
use r3d_mcp::Server;
use r3d_mcp::attach::{AgentState, ENDPOINT_FILE, Endpoint, Host};
use serde_json::{Value, json};

fn send(stream: &mut TcpStream, message: Value) {
    let mut line = message.to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).unwrap();
}

fn receive(reader: &mut BufReader<TcpStream>) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

/// Connect with `token` and open an MCP session.
fn connect(port: u16, token: &str) -> (TcpStream, BufReader<TcpStream>) {
    let mut agent = TcpStream::connect(("127.0.0.1", port)).unwrap();
    agent
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut reader = BufReader::new(agent.try_clone().unwrap());
    agent
        .write_all(format!("R3D-ATTACH {token}\n").as_bytes())
        .unwrap();
    send(
        &mut agent,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
    );
    let init = receive(&mut reader);
    assert_eq!(init["result"]["serverInfo"]["name"], "rustle3d", "{init}");
    send(
        &mut agent,
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    (agent, reader)
}

fn create_rect(agent: &mut TcpStream, reader: &mut BufReader<TcpStream>) {
    send(
        agent,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "panel_create_rect", "arguments": {"width": 100, "height": 200}}}),
    );
    let created = receive(reader);
    assert_eq!(created["result"]["isError"], false, "{created}");
}

#[test]
fn agents_edit_the_shared_project_until_disconnected() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = Arc::new(Mutex::new(
        Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap(),
    ));
    let changes = Arc::new(AtomicUsize::new(0));
    let counted = changes.clone();
    let server = Server::shared(
        ws.clone(),
        Some(Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        })),
    );
    let support = tmp.path().join("support");
    let host = Host::start(server, &support, "P").unwrap();
    let endpoint: Endpoint =
        serde_json::from_slice(&std::fs::read(support.join(ENDPOINT_FILE)).unwrap()).unwrap();
    assert_eq!(host.state(), AgentState::Waiting);

    // A wrong token is refused.
    let mut intruder = TcpStream::connect(("127.0.0.1", endpoint.port)).unwrap();
    intruder.write_all(b"R3D-ATTACH guess\n").unwrap();
    let mut answer = String::new();
    BufReader::new(intruder).read_line(&mut answer).unwrap();
    assert_eq!(answer.trim(), "denied");

    // The right one opens an MCP session on the shared project.
    let (mut agent, mut reader) = connect(endpoint.port, &endpoint.token);
    create_rect(&mut agent, &mut reader);
    assert_eq!(ws.lock().unwrap().session().project().panels().len(), 1);
    assert_eq!(changes.load(Ordering::SeqCst), 1);
    assert_eq!(host.state(), AgentState::Connected);

    // A second agent (an assistant's other session) is served alongside,
    // without waiting for the first to leave.
    let (mut other, mut other_reader) = connect(endpoint.port, &endpoint.token);
    create_rect(&mut other, &mut other_reader);
    create_rect(&mut agent, &mut reader);
    assert_eq!(ws.lock().unwrap().session().project().panels().len(), 3);

    // The user disconnects them: both connections close.
    host.disconnect();
    let deadline = Instant::now() + Duration::from_secs(5);
    while host.state() == AgentState::Connected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(host.state(), AgentState::Waiting);
    for reader in [&mut reader, &mut other_reader] {
        let mut rest = String::new();
        assert_eq!(reader.read_line(&mut rest).unwrap_or(0), 0, "{rest}");
    }

    // Not accepting agents any more: the endpoint is gone.
    drop(host);
    assert!(!support.join(ENDPOINT_FILE).exists());
}
