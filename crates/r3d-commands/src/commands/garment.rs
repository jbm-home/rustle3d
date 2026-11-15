use r3d_core::{Fabric, Garment, GarmentId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, Invalid};
use crate::spec::{CommandOutput, CommandSpec, Described};

fn garment(
    tx: &Transaction<'_>,
    id: Option<GarmentId>,
) -> Result<(GarmentId, Garment), CommandError> {
    let id = match id {
        Some(id) => id,
        None => tx
            .project()
            .default_garment()
            .ok_or_else(|| CommandError::Internal {
                detail: "project has no garment".into(),
            })?,
    };
    let g = tx
        .project()
        .garments()
        .get(&id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))?;
    Ok((id, g))
}

/// Choose the garment's fabric.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentSetFabric {
    /// Defaults to the project's garment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
    /// `cotton`, `denim`, `jersey`, `silk` or `wool`.
    pub fabric: Fabric,
}

impl Described for GarmentSetFabric {
    const SUMMARY: &'static str = "Choose what the garment is made of: cotton, denim, jersey, silk or wool. Sets how it drapes and how it looks.";
}

impl CommandSpec for GarmentSetFabric {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (id, g) = garment(tx, self.garment)?;
        tx.put_garment(
            id,
            Garment {
                fabric: self.fabric,
                ..g
            },
        );
        Ok(CommandOutput::default())
    }
}

/// Choose the garment's colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentSetColor {
    /// Defaults to the project's garment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
    /// `#RRGGBB`, or omitted to use the fabric's usual colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Described for GarmentSetColor {
    const SUMMARY: &'static str =
        "Set the garment's colour as #RRGGBB, or clear it to use the fabric's usual colour.";
}

impl CommandSpec for GarmentSetColor {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if let Some(c) = &self.color {
            let ok =
                c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit());
            if !ok {
                return Err(CommandError::invalid("color", Invalid::Format));
            }
        }
        let (id, g) = garment(tx, self.garment)?;
        tx.put_garment(
            id,
            Garment {
                color: self.color.clone(),
                ..g
            },
        );
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("garment.set_color".into())
    }
}
