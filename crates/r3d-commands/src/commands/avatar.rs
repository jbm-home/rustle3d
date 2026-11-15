use r3d_core::{Avatar, Body, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, Invalid, finite, non_blank};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Accepted ranges (cm), generous enough for adults of any build.
pub const HEIGHT_RANGE: (f64, f64) = (120.0, 220.0);
pub const BUST_RANGE: (f64, f64) = (60.0, 170.0);
pub const WAIST_RANGE: (f64, f64) = (45.0, 170.0);
pub const HIPS_RANGE: (f64, f64) = (60.0, 180.0);

fn in_range(field: &'static str, value: f64, (min, max): (f64, f64)) -> Result<(), CommandError> {
    finite(field, &[value])?;
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::Range(min, max)))
    }
}

pub(crate) fn validate_body(b: &Body) -> Result<(), CommandError> {
    in_range("masculinity", b.masculinity, (0.0, 1.0))?;
    in_range("height_cm", b.height_cm, HEIGHT_RANGE)?;
    in_range("bust_cm", b.bust_cm, BUST_RANGE)?;
    in_range("waist_cm", b.waist_cm, WAIST_RANGE)?;
    in_range("hips_cm", b.hips_cm, HIPS_RANGE)
}

/// Set the avatar's measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarSet {
    /// Measurements in cm; `masculinity` 0 = feminine body, 1 = masculine.
    pub body: Body,
    /// Name of the preset the measurements come from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

impl Described for AvatarSet {
    const SUMMARY: &'static str = "Set the avatar's body: masculinity (0 feminine – 1 masculine) and height, bust, waist and hips in cm. The 3D body is fitted to these measurements; avatar.get reports what it actually measures.";
}

impl CommandSpec for AvatarSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        validate_body(&self.body)?;
        if let Some(p) = &self.preset {
            non_blank("preset", p)?;
        }
        tx.set_avatar(Some(Avatar {
            body: self.body,
            preset: self.preset.as_deref().map(str::trim).map(str::to_owned),
        }));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("avatar.set".into())
    }
}

/// Remove the avatar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarRemove {}

impl Described for AvatarRemove {
    const SUMMARY: &'static str = "Remove the avatar from the project.";
}

impl CommandSpec for AvatarRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.set_avatar(None);
        Ok(CommandOutput::default())
    }
}
