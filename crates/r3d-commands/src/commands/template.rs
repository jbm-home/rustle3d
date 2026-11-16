//! Start a garment from a template.

use r3d_core::Transaction;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CommandError;
use crate::spec::{CommandOutput, CommandSpec, Described};
use crate::templates::{Sizes, Template, draft};

/// Draft a starter garment from body measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateApply {
    /// `t_shirt`, `straight_skirt` or `a_line_skirt`.
    pub template: Template,
    /// Sizes in cm; any left out come from the avatar (see template.list).
    #[serde(default)]
    pub sizes: Sizes,
}

impl Described for TemplateApply {
    const SUMMARY: &'static str = "Draft a starter garment (t_shirt, straight_skirt, a_line_skirt) from sizes in cm, the avatar's measurements filling in any left out: its pieces are drawn, sewn, placed around the body and given a fabric, ready to simulate and edit. A project without panels takes the template's name. Returns the panels, seams and notches.";
}

impl CommandSpec for TemplateApply {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        Ok(CommandOutput {
            created: draft(tx, self.template, self.sizes)?,
        })
    }
}
