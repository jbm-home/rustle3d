//! Command errors: structured (code + args) for translation, with an
//! English `Display` fallback. See `docs/architecture.md` §7.7.

use r3d_core::{AnyId, Localizable, Message};

/// Why an argument is invalid.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
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
    #[error("must not be negative")]
    Negative,
    #[error("must be a non-zero vector")]
    ZeroVector,
    #[error("must be strictly between 0 and the edge length ({0:.3} mm)")]
    OutsideEdge(f64),
    #[error("is not a linked mirror copy")]
    NotLinked,
    #[error("must be on the same panel")]
    OtherPanel,
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
            Invalid::Negative => "invalid.negative",
            Invalid::ZeroVector => "invalid.zero_vector",
            Invalid::OutsideEdge(_) => "invalid.outside_edge",
            Invalid::NotLinked => "invalid.not_linked",
            Invalid::OtherPanel => "invalid.other_panel",
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
    #[error("{panel} is a linked mirror copy: edit its source, or unlink it first")]
    LinkedMirror { panel: AnyId },
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
    #[error("`{name}` writes files and needs a project folder")]
    NeedsProject { name: &'static str },
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
                    Invalid::OutsideEdge(len) => m.arg("length_mm", format!("{len:.3}")),
                    _ => m,
                }
            }
            Self::ConfirmationRequired { command } => {
                Message::new("confirmation_required", text).arg("command", command)
            }
            Self::LinkedMirror { panel } => Message::new("linked_mirror", text).arg("panel", panel),
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
            Self::NeedsProject { name } => Message::new("needs_project", text).arg("name", name),
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

/// Check a finite, non-negative number.
pub(crate) fn non_negative(field: &'static str, value: f64) -> Result<(), CommandError> {
    finite(field, &[value])?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::Negative))
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
