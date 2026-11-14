//! Traits implemented by every command and query.

use r3d_core::{AnyId, Project, Transaction};
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::CommandError;

/// The stable, public name of a command or query (`"panel.create_rect"`).
/// Implemented by the registry macros, so the name is written exactly once.
pub trait Named {
    const NAME: &'static str;
}

/// Shared description, used for docs, CLI help and MCP tool descriptions.
pub trait Described {
    /// One-line summary.
    const SUMMARY: &'static str;
}

/// A document edit.
pub trait CommandSpec:
    Named + Described + Serialize + DeserializeOwned + JsonSchema + Clone + PartialEq
{
    /// Destructive commands need an explicit confirmation (`confirm: true`).
    const DESTRUCTIVE: bool = false;

    /// Validate and apply. Errors leave the document untouched: the session
    /// drops the transaction, which rolls back partial changes.
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError>;

    /// Consecutive commands with the same key merge into one undo step
    /// (e.g. dragging the same point).
    fn merge_key(&self) -> Option<String> {
        None
    }

    /// For destructive commands: whether the caller confirmed.
    fn confirmed(&self) -> bool {
        false
    }
}

/// A read-only question about the document.
pub trait QuerySpec: Named + Described + Serialize + DeserializeOwned + JsonSchema + Clone {
    fn run(&self, project: &Project) -> Result<Value, CommandError>;
}

/// What a command reports back.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CommandOutput {
    /// IDs of created entities, most important first (e.g. the panel, then
    /// its points, then its edges).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub created: Vec<AnyId>,
}

/// Registry entry: everything a front-end needs to expose an operation.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub name: &'static str,
    pub summary: &'static str,
    pub destructive: bool,
    /// JSON schema of the parameters (without the `type` field).
    pub schema: fn() -> Value,
}

pub(crate) fn schema_value<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null)
}

/// Serialize a query result.
pub(crate) fn to_value<T: Serialize>(value: &T) -> Result<Value, CommandError> {
    serde_json::to_value(value).map_err(|e| CommandError::Internal {
        detail: e.to_string(),
    })
}
