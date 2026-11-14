use r3d_core::{InternalLine, InternalLineId, PanelId, PathSegment, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, editable_panel, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// One segment of an internal line, starting where the previous one ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SegmentSpec {
    Line { to: Vec2 },
    Cubic { c1: Vec2, c2: Vec2, to: Vec2 },
}

/// Apply `f` to every point of a line (used for translation and mirroring).
pub(crate) fn mapped(line: &InternalLine, f: impl Fn(DVec2) -> DVec2) -> InternalLine {
    InternalLine {
        panel: line.panel,
        start: f(line.start),
        segments: line
            .segments
            .iter()
            .map(|s| match s {
                PathSegment::Line { to } => PathSegment::Line { to: f(*to) },
                PathSegment::Cubic { c1, c2, to } => PathSegment::Cubic {
                    c1: f(*c1),
                    c2: f(*c2),
                    to: f(*to),
                },
            })
            .collect(),
    }
}

pub(crate) fn translated(line: &InternalLine, d: DVec2) -> InternalLine {
    mapped(line, |p| p + d)
}

/// Draw an open path inside a panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InternalLineAdd {
    pub panel: PanelId,
    /// Start point (mm).
    pub start: Vec2,
    /// At least one segment.
    pub segments: Vec<SegmentSpec>,
}

impl Described for InternalLineAdd {
    const SUMMARY: &'static str = "Draw an internal line (an open path of straight and cubic segments, in mm) on a panel, e.g. a fold or placement line. Returns the line.";
}

impl CommandSpec for InternalLineAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        editable_panel(tx, self.panel)?;
        finite("start", &self.start)?;
        if self.segments.is_empty() {
            return Err(CommandError::invalid("segments", Invalid::Empty));
        }
        let segments = self
            .segments
            .iter()
            .map(|s| match s {
                SegmentSpec::Line { to } => {
                    finite("segments", to).map(|()| PathSegment::Line { to: v(*to) })
                }
                SegmentSpec::Cubic { c1, c2, to } => {
                    finite("segments", &[c1[0], c1[1], c2[0], c2[1], to[0], to[1]]).map(|()| {
                        PathSegment::Cubic {
                            c1: v(*c1),
                            c2: v(*c2),
                            to: v(*to),
                        }
                    })
                }
            })
            .collect::<Result<_, _>>()?;
        let id = tx.new_internal_line_id();
        tx.put_internal_line(
            id,
            InternalLine {
                panel: self.panel,
                start: v(self.start),
                segments,
            },
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Remove an internal line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InternalLineRemove {
    pub line: InternalLineId,
}

impl Described for InternalLineRemove {
    const SUMMARY: &'static str = "Remove an internal line.";
}

impl CommandSpec for InternalLineRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let line = tx
            .project()
            .internal_line(self.line)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.line))?;
        editable_panel(tx, line.panel)?;
        tx.remove_internal_line(self.line);
        Ok(CommandOutput::default())
    }
}
