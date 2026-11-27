//! Projects that make an object rather than a garment worn (spec 36).

use r3d_core::object::STUFFING;
use r3d_core::{ObjectSetup, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Refused in a project that makes an object: it has no body.
pub(crate) fn wearable(tx: &Transaction<'_>, field: &'static str) -> Result<(), CommandError> {
    match tx.project().object() {
        Some(_) => Err(CommandError::invalid(field, Invalid::Object)),
        None => Ok(()),
    }
}

/// Make the project an object, or change its stuffing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObjectSet {
    /// How firmly the parts closed all around are stuffed: 0 none, 1
    /// firm. Left out: as it is, or 0.5 for a project becoming an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stuffing: Option<f64>,
}

impl Described for ObjectSet {
    const SUMMARY: &'static str = "Make the project an object rather than a garment worn (a bag, a cushion, a toy): it has no body (the avatar and its measurements are removed), and in 3D it is weightless, so the cloth keeps the shape its seams give it instead of falling in folds; the parts closed all around are stuffed by `stuffing` (0 none – 1 firm, default 0.5; open parts never are). Again on an object, it changes the stuffing. object.clear makes it a garment again.";
}

impl CommandSpec for ObjectSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let current = tx.project().object().copied();
        let stuffing = match self.stuffing {
            Some(s) => {
                finite("stuffing", &[s])?;
                if !STUFFING.contains(&s) {
                    return Err(CommandError::invalid(
                        "stuffing",
                        Invalid::Range(*STUFFING.start(), *STUFFING.end()),
                    ));
                }
                s
            }
            None => current.unwrap_or(ObjectSetup::DEFAULT).stuffing,
        };
        let setup = ObjectSetup { stuffing };
        if tx.project().avatar().is_some() {
            tx.set_avatar(None);
        }
        if current != Some(setup) {
            tx.set_object(Some(setup));
        }
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("object.set".into())
    }
}

/// Make the project a garment worn again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObjectClear {}

impl Described for ObjectClear {
    const SUMMARY: &'static str = "Make an object project a garment worn again: weight comes back in 3D; give it a body with avatar.set.";
}

impl CommandSpec for ObjectClear {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if tx.project().object().is_some() {
            tx.set_object(None);
        }
        Ok(CommandOutput::default())
    }
}
