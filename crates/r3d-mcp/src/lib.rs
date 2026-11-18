//! MCP server for Rustle3D.
//!
//! Exposes the command layer to AI agents. Tools are generated from the
//! command, query and action registries (one source of truth with the CLI
//! and `docs/commands.md`): `panel.create_rect` becomes the tool
//! `panel_create_rect`, with the same JSON schema. `history_undo`,
//! `history_redo` and `batch` complete the set, and `render_board`,
//! `render_3d` and `render_photo` return PNG images so the agent can look
//! at its work.
//!
//! Headless (M2): one project folder over stdio, saved after every
//! successful change. Attached (M9, [`attach`]): the desktop app serves its
//! open project the same way to one agent at a time, and shows each change
//! live. See `docs/specs/09-mcp-connector.md`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub mod attach;
pub mod render;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use r3d_commands::{
    Action, Command, CommandError, Info, Progress, Query, Status, Step, Workspace, WorkspaceError,
    step,
};
use r3d_core::{Localizable, Message};
use r3d_render::Gpu;
use r3d_render::photo;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ProgressNotificationParam, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Map, Value, json};

const INSTRUCTIONS: &str = "Rustle3D: 2D sewing-pattern editor. Units are millimetres; the board's y axis points up. \
IDs are strings such as \"panel:2\" or \"point:3\"; commands return the IDs they create. \
Use project_summary and panel_get to inspect, the panel_*/point_*/edge_*/notch_* tools to edit, \
batch to apply several commands atomically, history_undo/history_redo to step back. \
Every successful change is saved to the project folder. \
export_pdf / export_svg write 1:1 patterns under exports/ (paths stay inside the project); \
export_gltf / export_obj write the draped garment (after simulate) for 3D tools and game engines. \
To dress the avatar: avatar_set, panel_place each panel around the body, seam_add, garment_set_fabric, \
then simulate (headless drape, cached; with `motion` the avatar walks, turns or raises its arms and the \
cloth follows) and render_3d to see the result; render_board shows the pattern; \
render_photo makes a photorealistic studio image of the draped garment (quality quick for a first look). \
Errors carry a stable `code`, `args` and an English `text`.";

/// Errors starting or running the server.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("cannot start the async runtime: {0}")]
    Runtime(std::io::Error),
    #[error("MCP transport error: {0}")]
    Transport(String),
    #[error("Rustle3D is not open with agents allowed (looked for {looked:?})")]
    NotAttached { looked: Vec<std::path::PathBuf> },
}

/// What a tool name maps to.
#[derive(Clone, Debug)]
enum Target {
    /// A registry operation, by its dotted name.
    Op(&'static str),
    Undo,
    Redo,
    List,
    GoTo,
    Batch,
    Render(render::View),
}

/// Called after a tool changed the project (the app refreshes its views).
pub type OnChange = Arc<dyn Fn() + Send + Sync>;

/// The MCP server over one project folder.
#[derive(Clone)]
pub struct Server {
    ws: Arc<Mutex<Workspace>>,
    on_change: Option<OnChange>,
    /// Created on the first render.
    gpu: Arc<Mutex<Option<Gpu>>>,
    tools: Arc<Vec<Tool>>,
    targets: Arc<BTreeMap<String, Target>>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("tools", &self.tools.len())
            .finish()
    }
}

/// MCP tool names allow `[a-zA-Z0-9_-]`: dots become underscores.
pub fn tool_name(op: &str) -> String {
    op.replace('.', "_")
}

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(mut m) => {
            m.remove("$schema");
            m
        }
        _ => Map::new(),
    }
}

fn no_params() -> Map<String, Value> {
    object(json!({ "type": "object", "properties": {}, "additionalProperties": false }))
}

fn catalog() -> (Vec<Tool>, BTreeMap<String, Target>) {
    let mut tools = Vec::new();
    let mut targets = BTreeMap::new();
    let mut add = |name: String, description: String, schema: Map<String, Value>, target| {
        tools.push(Tool::new(name.clone(), description, Arc::new(schema)));
        targets.insert(name, target);
    };
    let registries: [(Vec<Info>, &str); 3] = [
        (Command::registry(), "Command (undoable)."),
        (Query::registry(), "Query (read-only)."),
        (
            Action::registry(),
            "Action (writes a file inside the project).",
        ),
    ];
    for (infos, kind) in registries {
        for info in infos {
            let mut description = format!("{} {kind}", info.summary);
            if info.destructive {
                description.push_str(" Destructive: pass \"confirm\": true.");
            }
            add(
                tool_name(info.name),
                description,
                object((info.schema)()),
                Target::Op(info.name),
            );
        }
    }
    add(
        tool_name(step::UNDO),
        "Undo the last change.".into(),
        no_params(),
        Target::Undo,
    );
    add(
        tool_name(step::REDO),
        "Redo the last undone change.".into(),
        no_params(),
        Target::Redo,
    );
    add(
        tool_name(step::LIST),
        "The steps of the undo history, oldest first (`steps`, each with its `label`), and how          many are done (`at`)."
            .into(),
        no_params(),
        Target::List,
    );
    add(
        tool_name(step::GO_TO),
        "Undo or redo until `at` steps of the history are done (see history_list; 0 undoes          them all). Returns the panels changed."
            .into(),
        object(json!({
            "type": "object",
            "properties": { "at": { "type": "integer", "minimum": 0 } },
            "required": ["at"],
            "additionalProperties": false
        })),
        Target::GoTo,
    );
    add(
        step::BATCH.into(),
        "Run several commands atomically as one undo step: if one fails, none is applied and the \
         error gives its index. Each item is a command object with its dotted `type`, e.g. \
         {\"type\": \"panel.rename\", \"panel\": \"panel:2\", \"name\": \"Front\"}."
            .into(),
        object(json!({
            "type": "object",
            "properties": {
                "label": { "type": "string", "description": "The step's name in the history (default `batch`)" },
                "commands": { "type": "array", "items": { "type": "object" } }
            },
            "required": ["commands"],
            "additionalProperties": false
        })),
        Target::Batch,
    );
    let size_schema = || {
        object(json!({
            "type": "object",
            "properties": {
                "width": { "type": "integer", "minimum": 16, "maximum": 4096, "default": 1024 },
                "height": { "type": "integer", "minimum": 16, "maximum": 4096, "default": 768 }
            },
            "additionalProperties": false
        }))
    };
    add(
        "render_board".into(),
        "Render the 2D pattern (all panels, fitted to the image) as a PNG image.".into(),
        size_schema(),
        Target::Render(render::View::Board),
    );
    add(
        "render_3d".into(),
        "Render the garment on the avatar as a PNG image: the last `simulate` result if the \
         project has not changed since, otherwise the panels where the simulation starts. The \
         text result says which (`drape`: simulated or start)."
            .into(),
        size_schema(),
        Target::Render(render::View::Garment),
    );
    let photo_schema = {
        let names = |all: &[&str]| json!(all);
        object(json!({
            "type": "object",
            "properties": {
                "width": { "type": "integer", "minimum": 16, "maximum": 4096, "default": 1024 },
                "height": { "type": "integer", "minimum": 16, "maximum": 4096, "default": 768 },
                "quality": {
                    "enum": names(&photo::Quality::ALL.map(photo::Quality::name)),
                    "default": "good",
                    "description": "quick: 16 samples per pixel (seconds); good: 128; best: 512"
                },
                "lighting": {
                    "enum": names(&photo::Lighting::ALL.map(photo::Lighting::name)),
                    "default": "studio",
                    "description": "studio: key, fill and rim lights on a light grey backdrop; soft: large soft boxes, white backdrop; dramatic: one hard light on a dark backdrop"
                },
                "camera": {
                    "enum": names(&render::CameraAngle::ALL.map(render::CameraAngle::name)),
                    "default": "three_quarter"
                },
                "frame": {
                    "enum": ["garment", "body"],
                    "default": "garment",
                    "description": "Fill the image with the garment, or show the whole body"
                },
                "samples": { "type": "integer", "minimum": 1, "maximum": photo::MAX_SAMPLES, "description": "Overrides the quality's samples per pixel" },
                "bounces": { "type": "integer", "minimum": 0, "maximum": photo::MAX_BOUNCES, "description": "Overrides the quality's light bounces" },
                "denoise": { "type": "boolean", "default": true }
            },
            "additionalProperties": false
        }))
    };
    add(
        "render_photo".into(),
        "Render a photorealistic PNG of the garment on the avatar in a photo studio (path \
         traced, denoised): the last `simulate` result if the project has not changed since, \
         otherwise the panels where the simulation starts. Takes seconds (quick) to minutes \
         (best) depending on the GPU and the size; the text result gives the time taken."
            .into(),
        photo_schema,
        Target::Render(render::View::Photo(render::PhotoOptions::default())),
    );
    (tools, targets)
}

fn error_result(message: &Message) -> CallToolResult {
    let body = serde_json::to_string(&json!({ "error": message })).unwrap_or_default();
    CallToolResult::error(vec![ContentBlock::text(body)])
}

impl Server {
    pub fn new(ws: Workspace) -> Self {
        Self::shared(Arc::new(Mutex::new(ws)), None)
    }

    /// A server over a project someone else holds too (the app), calling
    /// `on_change` after each change it makes.
    pub fn shared(ws: Arc<Mutex<Workspace>>, on_change: Option<OnChange>) -> Self {
        let (tools, targets) = catalog();
        Self {
            ws,
            on_change,
            gpu: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
            targets: Arc::new(targets),
        }
    }

    /// Tool names, in catalog order.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name.to_string()).collect()
    }

    /// Execute a tool call; tool errors are returned as structured results
    /// the agent can read, never as protocol errors.
    pub fn call(&self, name: &str, arguments: Option<Map<String, Value>>) -> CallToolResult {
        self.call_with(name, arguments, &Progress::default())
    }

    /// [`Server::call`], a long step reporting to `progress` and stopping
    /// when it asks.
    pub fn call_with(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
        progress: &Progress,
    ) -> CallToolResult {
        let args = arguments.unwrap_or_default();
        let type_name = match self.targets.get(name) {
            None => {
                return error_result(&CommandError::UnknownType { name: name.into() }.message());
            }
            Some(Target::Render(view)) => {
                return match self.render(*view, &args) {
                    Ok((png, info)) => CallToolResult::success(vec![
                        ContentBlock::image(STANDARD.encode(png), "image/png"),
                        ContentBlock::text(info.to_string()),
                    ]),
                    Err(e) => error_result(&e.message()),
                };
            }
            Some(Target::Op(op)) => *op,
            Some(Target::Undo) => step::UNDO,
            Some(Target::Redo) => step::REDO,
            Some(Target::List) => step::LIST,
            Some(Target::GoTo) => step::GO_TO,
            Some(Target::Batch) => step::BATCH,
        };
        match self.run_step(type_name, args, progress) {
            Ok(value) => {
                let body = serde_json::to_string(&value).unwrap_or_default();
                CallToolResult::success(vec![ContentBlock::text(body)])
            }
            Err(e) => error_result(&e.message()),
        }
    }

    fn render(
        &self,
        view: render::View,
        args: &Map<String, Value>,
    ) -> Result<(Vec<u8>, Value), render::RenderError> {
        let size = |key: &str, default: u32| -> Result<u32, render::RenderError> {
            match args.get(key) {
                None => Ok(default),
                Some(v) => v
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| CommandError::Malformed {
                        name: key.into(),
                        detail: "expected a whole number of pixels".into(),
                    })
                    .map_err(Into::into),
            }
        };
        let (view, extra): (render::View, &[&str]) = match view {
            render::View::Photo(_) => (
                render::View::Photo(render::PhotoOptions::from_args(args)?),
                &render::PhotoOptions::KEYS,
            ),
            other => (other, &[]),
        };
        if let Some(key) = args
            .keys()
            .find(|k| *k != "width" && *k != "height" && !extra.contains(&k.as_str()))
        {
            let mut expected = vec!["width", "height"];
            expected.extend_from_slice(extra);
            return Err(CommandError::Malformed {
                name: key.clone(),
                detail: format!("unknown parameter (expected {})", expected.join(", ")),
            }
            .into());
        }
        let (width, height) = (size("width", 1024)?, size("height", 768)?);
        let internal = |detail: &str| CommandError::Internal {
            detail: detail.into(),
        };
        let mut gpu = self.gpu.lock().map_err(|_| internal("GPU lock poisoned"))?;
        if gpu.is_none() {
            *gpu = Some(render::headless_gpu()?);
        }
        let gpu = gpu.as_ref().ok_or_else(|| internal("no GPU"))?;
        let ws = self
            .ws
            .lock()
            .map_err(|_| internal("workspace lock poisoned"))?;
        render::render(gpu, &ws, view, width, height)
    }

    /// Run the step `type_name` with `args`, saving after a change.
    fn run_step(
        &self,
        type_name: &str,
        mut args: Map<String, Value>,
        progress: &Progress,
    ) -> Result<Value, WorkspaceError> {
        args.insert("type".into(), Value::from(type_name));
        let step = Step::from_value(Value::Object(args))?;
        let mut ws = self.ws.lock().map_err(|_| {
            WorkspaceError::Command(r3d_commands::CommandError::Internal {
                detail: "workspace lock poisoned".into(),
            })
        })?;
        let value = ws.run_step_with(&step, progress)?;
        let changed = ws.session().is_dirty();
        if changed {
            ws.save()?;
        }
        drop(ws);
        if changed && let Some(notify) = &self.on_change {
            notify();
        }
        Ok(value)
    }
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        let mut implementation = Implementation::from_build_env();
        implementation.name = "rustle3d".into();
        implementation.version = env!("CARGO_PKG_VERSION").into();
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(implementation)
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.tools.as_ref().clone()))
    }

    /// Long steps (a graded template, a drape) send progress notifications
    /// when the client asked for them (a progress token), and stop when
    /// the client cancels the request. The step runs off the async
    /// threads.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let cancel = Arc::new(AtomicBool::new(false));
        // Ends when the client cancels, or when the call is over.
        let over = context.ct.child_token();
        let (waiting, flag) = (over.clone(), cancel.clone());
        tokio::spawn(async move {
            waiting.cancelled().await;
            flag.store(true, Ordering::Relaxed);
        });
        let progress = match context.meta.get_progress_token() {
            Some(token) => {
                let (peer, runtime) = (context.peer.clone(), tokio::runtime::Handle::current());
                Progress::new(
                    move |s: &Status| {
                        let mut note = ProgressNotificationParam::new(token.clone(), s.done.into());
                        note.total = Some(s.total.into());
                        note.message = Some(if s.item.is_empty() {
                            s.task.to_owned()
                        } else {
                            format!("{} {}", s.task, s.item)
                        });
                        let peer = peer.clone();
                        runtime.spawn(async move {
                            // A client gone away: nothing to tell.
                            let _ = peer.notify_progress(note).await;
                        });
                    },
                    cancel,
                )
            }
            None => Progress::new(|_: &Status| {}, cancel),
        };
        let server = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            server.call_with(&request.name, request.arguments, &progress)
        })
        .await
        .unwrap_or_else(|e| {
            error_result(
                &CommandError::Internal {
                    detail: e.to_string(),
                }
                .message(),
            )
        });
        over.cancel();
        Ok(result.into())
    }
}

/// Serve `ws` over stdin/stdout until the client disconnects.
pub fn serve_stdio(ws: Workspace) -> Result<(), ServeError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ServeError::Runtime)?;
    runtime.block_on(async {
        let service = Server::new(ws)
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| ServeError::Transport(e.to_string()))?;
        service
            .waiting()
            .await
            .map_err(|e| ServeError::Transport(e.to_string()))?;
        Ok(())
    })
}
