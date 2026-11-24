//! Start a garment from a template.

use r3d_core::Transaction;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CommandError;
use crate::progress::Progress;
use crate::spec::{CommandOutput, CommandSpec, Described};
use crate::templates::{Sizes, Template, draft};

/// Draft a starter garment from body measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateApply {
    /// `t_shirt`, `shirt`, `trousers`, `dress`, `straight_skirt`,
    /// `a_line_skirt` or `tote_bag`.
    pub template: Template,
    /// Sizes in cm; any left out come from the avatar (see template.list).
    #[serde(default)]
    pub sizes: Sizes,
}

impl Described for TemplateApply {
    const SUMMARY: &'static str = "Draft a starter garment (t_shirt, shirt, trousers, dress, straight_skirt, a_line_skirt, tote_bag) from sizes in cm, the avatar's measurements filling in any left out: its pieces are drawn, sewn, placed around the body and given a fabric, ready to simulate and edit. In a project without panels its garment takes the template's name (the project keeps its own); in a project with pieces the template becomes a garment of its own, worn over the others (see garment.set_layer). Returns the garment if one was added, then the panels, seams and notches.";
}

impl CommandSpec for TemplateApply {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    /// Graded, the template is drafted in every size (seconds): reported
    /// size by size.
    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        Ok(CommandOutput {
            created: draft(tx, self.template, self.sizes, progress)?,
        })
    }
}
