//! Script steps: what the CLI `run` subcommand and MCP batch accept.
//!
//! A step is a JSON object with a `type`: a command name, a query name,
//! `history.undo`, `history.redo`, or `batch` (`{"type": "batch",
//! "commands": [...]}`, executed atomically as one undo step).

use serde_json::Value;

use crate::error::CommandError;
use crate::registry::{Action, Command, Query};

pub const UNDO: &str = "history.undo";
pub const REDO: &str = "history.redo";
pub const BATCH: &str = "batch";

/// One step of a script.
#[derive(Clone, Debug)]
pub enum Step {
    Command(Command),
    Query(Query),
    /// Writes files; needs a project folder (see `Workspace`).
    Action(Action),
    Undo,
    Redo,
    Batch(Vec<Command>),
}

fn type_of(value: &Value) -> Result<String, CommandError> {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| CommandError::Malformed {
            name: "step".into(),
            detail: "expected an object with a string `type` field".into(),
        })
}

fn malformed(name: &str, e: serde_json::Error) -> CommandError {
    CommandError::Malformed {
        name: name.to_owned(),
        detail: e.to_string(),
    }
}

fn is_command(name: &str) -> bool {
    Command::registry().iter().any(|i| i.name == name)
}

fn is_query(name: &str) -> bool {
    Query::registry().iter().any(|i| i.name == name)
}

fn is_action(name: &str) -> bool {
    Action::registry().iter().any(|i| i.name == name)
}

/// Parse a command, with clear errors for unknown names.
pub fn parse_command(value: Value) -> Result<Command, CommandError> {
    let name = type_of(&value)?;
    if !is_command(&name) {
        return Err(CommandError::UnknownType { name });
    }
    serde_json::from_value(value).map_err(|e| malformed(&name, e))
}

/// Parse a query, with clear errors for unknown names.
pub fn parse_query(value: Value) -> Result<Query, CommandError> {
    let name = type_of(&value)?;
    if !is_query(&name) {
        return Err(CommandError::UnknownType { name });
    }
    serde_json::from_value(value).map_err(|e| malformed(&name, e))
}

impl Step {
    pub fn from_value(value: Value) -> Result<Step, CommandError> {
        let name = type_of(&value)?;
        match name.as_str() {
            UNDO => Ok(Step::Undo),
            REDO => Ok(Step::Redo),
            BATCH => {
                let Value::Object(mut obj) = value else {
                    return Err(CommandError::Malformed {
                        name,
                        detail: "expected an object".into(),
                    });
                };
                let Some(Value::Array(items)) = obj.remove("commands") else {
                    return Err(CommandError::Malformed {
                        name,
                        detail: "expected a `commands` array".into(),
                    });
                };
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| parse_command(v).map_err(|e| e.at(i)))
                    .collect::<Result<_, _>>()
                    .map(Step::Batch)
            }
            n if is_command(n) => parse_command(value).map(Step::Command),
            n if is_query(n) => parse_query(value).map(Step::Query),
            n if is_action(n) => serde_json::from_value(value)
                .map(Step::Action)
                .map_err(|e| malformed(&name, e)),
            _ => Err(CommandError::UnknownType { name }),
        }
    }

    /// Parse a script: a JSON array of steps.
    pub fn parse_script(text: &str) -> Result<Vec<Step>, CommandError> {
        let value: Value = serde_json::from_str(text).map_err(|e| malformed("script", e))?;
        let Value::Array(items) = value else {
            return Err(CommandError::Malformed {
                name: "script".into(),
                detail: "expected a JSON array of steps".into(),
            });
        };
        items
            .into_iter()
            .enumerate()
            .map(|(i, v)| Step::from_value(v).map_err(|e| e.at(i)))
            .collect()
    }
}
