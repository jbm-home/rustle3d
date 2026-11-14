use r3d_core::{EdgeId, Notch, NotchId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::editable_edge_panel;
use crate::error::{CommandError, non_negative};
use crate::spec::{CommandOutput, CommandSpec, Described};

fn get_notch(tx: &Transaction<'_>, id: NotchId) -> Result<Notch, CommandError> {
    let notch = tx
        .project()
        .notch(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))?;
    editable_edge_panel(tx, notch.edge)?;
    Ok(notch)
}

/// Add a notch on an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotchAdd {
    pub edge: EdgeId,
    /// Distance from the edge's start point along the edge (mm, ≥ 0).
    pub offset_mm: f64,
}

impl Described for NotchAdd {
    const SUMMARY: &'static str = "Add a notch (alignment mark) on an edge, at a distance in mm from the edge's start point. Returns the notch.";
}

impl CommandSpec for NotchAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_negative("offset_mm", self.offset_mm)?;
        editable_edge_panel(tx, self.edge)?;
        let id = tx.new_notch_id();
        tx.put_notch(
            id,
            Notch {
                edge: self.edge,
                offset_mm: self.offset_mm,
            },
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Move a notch along its edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotchMove {
    pub notch: NotchId,
    /// New distance from the edge's start point (mm, ≥ 0).
    pub offset_mm: f64,
}

impl Described for NotchMove {
    const SUMMARY: &'static str =
        "Move a notch along its edge to a new distance (mm) from the edge's start point.";
}

impl CommandSpec for NotchMove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_negative("offset_mm", self.offset_mm)?;
        let notch = get_notch(tx, self.notch)?;
        tx.put_notch(
            self.notch,
            Notch {
                offset_mm: self.offset_mm,
                ..notch
            },
        );
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("notch.move:{}", self.notch))
    }
}

/// Remove a notch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotchRemove {
    pub notch: NotchId,
}

impl Described for NotchRemove {
    const SUMMARY: &'static str = "Remove a notch.";
}

impl CommandSpec for NotchRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_notch(tx, self.notch)?;
        tx.remove_notch(self.notch);
        Ok(CommandOutput::default())
    }
}
