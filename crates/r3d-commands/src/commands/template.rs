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
    /// `t_shirt`, `shirt`, `trousers`, `hoodie`, `joggers`, `jeans`,
    /// `shorts`, `dress`, `straight_skirt`, `a_line_skirt`, `tote_bag`,
    /// `sweatshirt`, `bodysuit`, `pyjama`, `pyjama_top`, `pyjama_bottoms`
    /// or `jogging_suit`.
    pub template: Template,
    /// Sizes in cm; any left out come from the avatar (see template.list).
    #[serde(default)]
    pub sizes: Sizes,
}

impl Described for TemplateApply {
    const SUMMARY: &'static str = "Draft a starter garment (t_shirt, shirt, trousers, hoodie, joggers, jeans, shorts, dress, straight_skirt, a_line_skirt, tote_bag, sweatshirt, and for babies and children bodysuit, pyjama_top, pyjama_bottoms, and the sets pyjama and jogging_suit (joggers and a sweatshirt), which add their bottoms and then their top worn over them as two garments, the given length and ease being the top's; library.list gives their kinds) from sizes in cm, the avatar's measurements filling in any left out: its pieces are drawn, sewn, placed around the body and given a fabric, ready to simulate and edit. In a project without panels its garment takes the template's name (the project keeps its own); in a project with pieces the template becomes a garment of its own, worn over the others (see garment.set_layer). Returns the garment if one was added, then the panels, seams and notches.";
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
        // A set's parts, each a garment: its top's length and ease are
        // the set's, its bottoms take their own.
        let mut created = Vec::new();
        let parts = self.template.parts();
        let top = parts.len() - 1;
        for (k, part) in parts.into_iter().enumerate() {
            let sizes = if k == top {
                self.sizes
            } else {
                Sizes {
                    length_cm: None,
                    ease_cm: None,
                    ..self.sizes
                }
            };
            created.extend(draft(tx, part, sizes, progress)?);
        }
        Ok(CommandOutput { created })
    }
}
