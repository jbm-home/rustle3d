//! Command errors: structured (code + args) for translation, with an
//! English `Display` fallback. See `docs/architecture.md` §7.7.

use r3d_core::{AnyId, Localizable, Message};

/// Why an argument is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Invalid {
    #[error("must be a finite number")]
    NotFinite,
    #[error("must be greater than zero")]
    NotPositive,
    #[error("needs at least {0} points")]
    TooFewPoints(usize),
    #[error("must form a simple outline (no crossing, overlapping or repeated edges)")]
    NotSimple,
    #[error("must not be empty")]
    Empty,
    #[error("must refer to two different points")]
    SamePoint,
}

impl Invalid {
    pub fn code(&self) -> &'static str {
        match self {
            Invalid::NotFinite => "invalid.not_finite",
            Invalid::NotPositive => "invalid.not_positive",
            Invalid::TooFewPoints(_) => "invalid.too_few_points",
            Invalid::NotSimple => "invalid.not_simple",
            Invalid::Empty => "invalid.empty",
            Invalid::SamePoint => "invalid.same_point",
        }
    }
}

/// Error from executing a step.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CommandError {
    #[error("{id} does not exist")]
    NotFound { id: AnyId },
    #[error("`{field}` {reason}")]
    Invalid {
        field: &'static str,
        reason: Invalid,
    },
    #[error("`{command}` is destructive: pass \"confirm\": true to run it")]
    ConfirmationRequired { command: &'static str },
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("unknown step type `{name}`")]
    UnknownType { name: String },
    #[error("malformed `{name}`: {detail}")]
    Malformed { name: String, detail: String },
    #[error("step {index} failed: {source}")]
    AtStep {
        index: usize,
        source: Box<CommandError>,
    },
    #[error("internal error: {detail}")]
    Internal { detail: String },
}

impl CommandError {
    pub fn invalid(field: &'static str, reason: Invalid) -> Self {
        CommandError::Invalid { field, reason }
    }

    pub fn not_found(id: impl Into<AnyId>) -> Self {
        CommandError::NotFound { id: id.into() }
    }

    pub(crate) fn at(self, index: usize) -> Self {
        CommandError::AtStep {
            index,
            source: Box::new(self),
        }
    }
}

impl Localizable for CommandError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::NotFound { id } => Message::new("not_found", text).arg("id", id),
            Self::Invalid { field, reason } => {
                let m = Message::new(reason.code(), text).arg("field", field);
                match reason {
                    Invalid::TooFewPoints(n) => m.arg("min", n),
                    _ => m,
                }
            }
            Self::ConfirmationRequired { command } => {
                Message::new("confirmation_required", text).arg("command", command)
            }
            Self::NothingToUndo => Message::new("nothing_to_undo", text),
            Self::NothingToRedo => Message::new("nothing_to_redo", text),
            Self::UnknownType { name } => Message::new("unknown_type", text).arg("name", name),
            Self::Malformed { name, detail } => Message::new("malformed", text)
                .arg("name", name)
                .arg("detail", detail),
            Self::AtStep { index, source } => {
                let mut m = source.message();
                m.args.insert("step", index.to_string());
                m.text = text;
                m
            }
            Self::Internal { detail } => Message::new("internal", text).arg("detail", detail),
        }
    }
}

/// Check that every number is finite.
pub(crate) fn finite(field: &'static str, values: &[f64]) -> Result<(), CommandError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::NotFinite))
    }
}

/// Check a strictly positive, finite number.
pub(crate) fn positive(field: &'static str, value: f64) -> Result<(), CommandError> {
    finite(field, &[value])?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::NotPositive))
    }
}

/// Check a name that is not blank.
pub(crate) fn non_blank(field: &'static str, value: &str) -> Result<(), CommandError> {
    if value.trim().is_empty() {
        Err(CommandError::invalid(field, Invalid::Empty))
    } else {
        Ok(())
    }
}
