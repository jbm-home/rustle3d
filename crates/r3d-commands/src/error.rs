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
    #[error("must be between {0} and {1}")]
    Range(f64, f64),
    #[error("has an invalid format")]
    Format,
    #[error("would turn a panel inside out")]
    Inverted,
    #[error("must be an end point of the seam")]
    NotSeamEnd,
    #[error("must be one of the seam's two edges")]
    NotSeamSide,
    #[error("must cross the panel's outline exactly twice, both ends outside the panel")]
    NotAcross,
    #[error("has a linked mirror copy: unlink it first")]
    Mirrored,
    #[error("is the project's only garment")]
    LastGarment,
    #[error("needs a size range: set one first (grading.set_sizes or grading.use_chart)")]
    NoSizes,
    #[error("is not a size of the range")]
    UnknownSize,
    #[error("needs at least two sizes")]
    TooFewSizes,
    #[error("must not repeat a name")]
    DuplicateName,
    #[error("gives a size a body outside the accepted measurements")]
    BodyOutOfRange,
    #[error("must be zero in the base size")]
    BaseOffset,
    #[error("needs either `step` or `offsets`")]
    StepOrOffsets,
    #[error("is symmetric: make its halves independent first (panel.unlink)")]
    Symmetric,
    #[error("must be a straight edge")]
    NotStraight,
    #[error("is not a dimension (distance, length or angle)")]
    NotDimension,
    #[error("is already constrained this way")]
    Duplicate,
    #[error("must name points and edges of the same piece's outline")]
    NotJoined,
    #[error("is graded by formulas: change them, or the sizes")]
    GradedByFormula,
    #[error("works on seams between two whole edges")]
    NotWholeEdges,
    #[error("overlaps a stretch already sewn")]
    Overlaps,
    #[error("would leave several pieces, or nothing")]
    Apart,
    #[error("has holes: fill them first (panel.remove_hole)")]
    HasHoles,
    #[error("must be inside the piece")]
    OutsidePiece,
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
            Invalid::Range(..) => "invalid.range",
            Invalid::Format => "invalid.format",
            Invalid::Inverted => "invalid.inverted",
            Invalid::NotSeamEnd => "invalid.not_seam_end",
            Invalid::NotSeamSide => "invalid.not_seam_side",
            Invalid::NotAcross => "invalid.not_across",
            Invalid::Mirrored => "invalid.mirrored",
            Invalid::LastGarment => "invalid.last_garment",
            Invalid::NoSizes => "invalid.no_sizes",
            Invalid::UnknownSize => "invalid.unknown_size",
            Invalid::TooFewSizes => "invalid.too_few_sizes",
            Invalid::DuplicateName => "invalid.duplicate_name",
            Invalid::BodyOutOfRange => "invalid.body_out_of_range",
            Invalid::BaseOffset => "invalid.base_offset",
            Invalid::StepOrOffsets => "invalid.step_or_offsets",
            Invalid::Symmetric => "invalid.symmetric",
            Invalid::NotStraight => "invalid.not_straight",
            Invalid::NotDimension => "invalid.not_dimension",
            Invalid::Duplicate => "invalid.duplicate",
            Invalid::NotJoined => "invalid.not_joined",
            Invalid::GradedByFormula => "invalid.graded_by_formula",
            Invalid::NotWholeEdges => "invalid.not_whole_edges",
            Invalid::Overlaps => "invalid.overlaps",
            Invalid::Apart => "invalid.apart",
            Invalid::HasHoles => "invalid.has_holes",
            Invalid::OutsidePiece => "invalid.outside_piece",
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
    #[error(
        "{id} follows other geometry (a linked mirror copy or the reflected half of a symmetric piece): edit what it follows"
    )]
    Derived { id: AnyId },
    #[error("{} ({})", .error, .of)]
    Formula {
        /// The constraint or the variable whose formula it is.
        of: String,
        error: r3d_core::formula::FormulaError,
    },
    #[error("the constraints cannot all hold ({}): nothing was changed", list(.constraints))]
    Unsolvable { constraints: Vec<AnyId> },
    #[error("{edge} is sewn ({seam}): remove the seam first")]
    Sewn { edge: AnyId, seam: AnyId },
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
    #[error("bad reference `{reference}`: {reason}")]
    BadReference {
        reference: String,
        reason: &'static str,
    },
    #[error("{panel} is wider than the fabric ({width_mm} mm) in every direction it may lie")]
    MarkerTooNarrow { panel: AnyId, width_mm: f64 },
    #[error("nothing to simulate: place at least one panel around the body (panel.place)")]
    NothingToSimulate,
    #[error("stopped before the end: nothing was changed")]
    Cancelled,
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
                    Invalid::Range(min, max) => m.arg("min", min).arg("max", max),
                    _ => m,
                }
            }
            Self::ConfirmationRequired { command } => {
                Message::new("confirmation_required", text).arg("command", command)
            }
            Self::LinkedMirror { panel } => Message::new("linked_mirror", text).arg("panel", panel),
            Self::Derived { id } => Message::new("derived", text).arg("id", id),
            Self::Formula { of, error } => {
                let mut m = error.message();
                m.args.insert("of", of.clone());
                m.text = text;
                m
            }
            Self::Unsolvable { constraints } => {
                Message::new("constraints_unsolvable", text).arg("constraints", list(constraints))
            }
            Self::Sewn { edge, seam } => Message::new("sewn", text)
                .arg("edge", edge)
                .arg("seam", seam),
            Self::NothingToUndo => Message::new("nothing_to_undo", text),
            Self::NothingToRedo => Message::new("nothing_to_redo", text),
            Self::NothingToSimulate => Message::new("nothing_to_simulate", text),
            Self::MarkerTooNarrow { panel, width_mm } => Message::new("marker_too_narrow", text)
                .arg("panel", panel)
                .arg("width_mm", width_mm),
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
            Self::BadReference { reference, reason } => Message::new("bad_reference", text)
                .arg("reference", reference)
                .arg("reason", reason),
            Self::Cancelled => Message::new("cancelled", text),
            Self::Internal { detail } => Message::new("internal", text).arg("detail", detail),
        }
    }
}

/// IDs as a comma-separated list.
fn list(ids: &[AnyId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
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
