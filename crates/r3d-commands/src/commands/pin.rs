use r3d_core::{PanelId, Pin, PinId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, get_panel, v};
use crate::error::{CommandError, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Pin the cloth in 3D.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinAdd {
    pub panel: PanelId,
    /// The pinned point on the pattern, `[x, y]` in mm.
    pub at: Vec2,
    /// Where it is held in 3D, `[x, y, z]` in metres (Y up, the avatar's
    /// feet at y = 0).
    pub position: [f64; 3],
}

impl Described for PinAdd {
    const SUMMARY: &'static str = "Pin the cloth: the panel's cloth within 3 cm of `at` (pattern mm) is held still at `position` (3D metres) during the simulation. Returns the pin.";
}

impl CommandSpec for PinAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_panel(tx.project(), self.panel)?;
        finite("at", &self.at)?;
        finite("position", &self.position)?;
        let id = tx.new_pin_id();
        tx.put_pin(
            id,
            Pin {
                panel: self.panel,
                at: v(self.at),
                position: self.position.into(),
            },
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Remove a pin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinRemove {
    pub pin: PinId,
}

impl Described for PinRemove {
    const SUMMARY: &'static str = "Remove a pin: the cloth there is free again.";
}

impl CommandSpec for PinRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.remove_pin(self.pin)
            .ok_or_else(|| CommandError::not_found(self.pin))?;
        Ok(CommandOutput::default())
    }
}
