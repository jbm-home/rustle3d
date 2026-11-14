//! MCP server for Rustle3D.
//!
//! Exposes the command layer to AI agents. Tools are generated from the
//! command, query and action registries (one source of truth with the CLI
//! and `docs/commands.md`): `panel.create_rect` becomes the tool
//! `panel_create_rect`, with the same JSON schema. `history_undo`,
//! `history_redo` and `batch` complete the set.
//!
//! M2 serves one project folder over stdio (headless), saving it after
//! every successful change. Attached mode (the desktop app hosting the same
//! server on a local socket) comes in M8. See `docs/specs/09-mcp-connector.md`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use r3d_commands::{Action, Command, Info, Query, Step, Workspace, WorkspaceError, step};
use r3d_core::{Localizable, Message};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Map, Value, json};

const INSTRUCTIONS: &str = "Rustle3D: 2D sewing-pattern editor. Units are millimetres; the board's y axis points up. \
IDs are strings such as \"panel:2\" or \"point:3\"; commands return the IDs they create. \
Use project_summary and panel_get to inspect, the panel_*/point_*/edge_*/notch_* tools to edit, \
batch to apply several commands atomically, history_undo/history_redo to step back. \
Every successful change is saved to the project folder. \
export_pdf / export_svg write 1:1 patterns under exports/ (paths stay inside the project). \
Errors carry a stable `code`, `args` and an English `text`.";

/// Errors starting or running the server.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("cannot start the async runtime: {0}")]
    Runtime(std::io::Error),
    #[error("MCP transport error: {0}")]
    Transport(String),
}

/// What a tool name maps to.
#[derive(Clone, Debug)]
enum Target {
    /// A registry operation, by its dotted name.
    Op(&'static str),
    Undo,
    Redo,
    Batch,
}

/// The MCP server over one project folder.
#[derive(Clone)]
pub struct Server {
    ws: Arc<Mutex<Workspace>>,
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
        step::BATCH.into(),
        "Run several commands atomically as one undo step: if one fails, none is applied and the \
         error gives its index. Each item is a command object with its dotted `type`, e.g. \
         {\"type\": \"panel.rename\", \"panel\": \"panel:2\", \"name\": \"Front\"}."
            .into(),
        object(json!({
            "type": "object",
            "properties": { "commands": { "type": "array", "items": { "type": "object" } } },
            "required": ["commands"],
            "additionalProperties": false
        })),
        Target::Batch,
    );
    (tools, targets)
}

fn error_result(message: &Message) -> CallToolResult {
    let body = serde_json::to_string(&json!({ "error": message })).unwrap_or_default();
    CallToolResult::error(vec![ContentBlock::text(body)])
}

impl Server {
    pub fn new(ws: Workspace) -> Self {
        let (tools, targets) = catalog();
        Self {
            ws: Arc::new(Mutex::new(ws)),
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
        match self.try_call(name, arguments) {
            Ok(value) => {
                let body = serde_json::to_string(&value).unwrap_or_default();
                CallToolResult::success(vec![ContentBlock::text(body)])
            }
            Err(e) => error_result(&e.message()),
        }
    }

    fn try_call(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<Value, WorkspaceError> {
        let target = self.targets.get(name).cloned().ok_or_else(|| {
            WorkspaceError::Command(r3d_commands::CommandError::UnknownType { name: name.into() })
        })?;
        let mut args = arguments.unwrap_or_default();
        let type_name = match target {
            Target::Op(op) => op,
            Target::Undo => step::UNDO,
            Target::Redo => step::REDO,
            Target::Batch => step::BATCH,
        };
        args.insert("type".into(), Value::from(type_name));
        let step = Step::from_value(Value::Object(args))?;
        let mut ws = self.ws.lock().map_err(|_| {
            WorkspaceError::Command(r3d_commands::CommandError::Internal {
                detail: "workspace lock poisoned".into(),
            })
        })?;
        let value = ws.run_step(&step)?;
        if ws.session().is_dirty() {
            ws.save()?;
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

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        Ok(self.call(&request.name, request.arguments).into())
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
