//! The engine a UI window talks to: one project folder.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use flutter_rust_bridge::frb;
use r3d_commands::{Step, Workspace};
use r3d_core::{Localizable, Message};

/// A structured, translatable error (see `docs/architecture.md` §7.7).
#[derive(Debug, Clone)]
pub struct ApiError {
    /// Stable code, the translation key.
    pub code: String,
    /// Named arguments for the translation.
    pub args: HashMap<String, String>,
    /// English fallback.
    pub text: String,
}

impl From<Message> for ApiError {
    fn from(m: Message) -> Self {
        Self {
            code: m.code.to_owned(),
            args: m.args.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            text: m.text,
        }
    }
}

fn err(e: impl Localizable) -> ApiError {
    e.message().into()
}

/// A project opened by the UI.
#[frb(opaque)]
pub struct Engine {
    ws: Mutex<Workspace>,
}

impl Engine {
    /// Open the project folder at `path`, creating it if it does not exist.
    #[frb(sync)]
    pub fn open(path: String) -> Result<Engine, ApiError> {
        let dir = Path::new(&path);
        let ws = if dir.exists() {
            Workspace::open(dir).map_err(err)?
        } else {
            let name = dir
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into());
            Workspace::create(dir, &name).map_err(err)?
        };
        Ok(Engine { ws: Mutex::new(ws) })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Workspace>, ApiError> {
        self.ws.lock().map_err(|_| ApiError {
            code: "internal".into(),
            args: HashMap::new(),
            text: "engine lock poisoned".into(),
        })
    }

    /// Run one step (a JSON object, as in scripts and MCP) and return its
    /// JSON result. Changes are saved immediately.
    #[frb(sync)]
    pub fn run(&self, step_json: String) -> Result<String, ApiError> {
        let value: serde_json::Value = serde_json::from_str(&step_json).map_err(|e| ApiError {
            code: "malformed".into(),
            args: HashMap::from([("detail".into(), e.to_string())]),
            text: format!("malformed step: {e}"),
        })?;
        let step = Step::from_value(value).map_err(err)?;
        let mut ws = self.lock()?;
        let result = ws.run_step(&step).map_err(err)?;
        if ws.session().is_dirty() {
            ws.save().map_err(err)?;
        }
        Ok(result.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_run_and_structured_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("P.r3d").to_string_lossy().into_owned();
        let engine = Engine::open(path.clone()).unwrap();
        let out = engine
            .run(r#"{"type":"panel.create_rect","width":100,"height":200}"#.into())
            .unwrap();
        assert!(out.contains("panel:2"));
        let e = engine
            .run(r#"{"type":"panel.create_rect","width":-1,"height":2}"#.into())
            .err()
            .unwrap();
        assert_eq!(e.code, "invalid.not_positive");
        // Saved: reopening sees the panel.
        drop(engine);
        let again = Engine::open(path).unwrap();
        assert!(
            again
                .run(r#"{"type":"panel.list"}"#.into())
                .unwrap()
                .contains("panel:2")
        );
    }
}
