use r3d_core::{AnyId, Fabric, Garment, GarmentId, PanelId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PanelDelete;
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
        check_color(self.color.as_deref())?;
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

/// Add a garment, worn over the others.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentAdd {
    pub name: String,
    /// Defaults to cotton.
    #[serde(default)]
    pub fabric: Fabric,
    /// `#RRGGBB`, or omitted to use the fabric's usual colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Described for GarmentAdd {
    const SUMMARY: &'static str = "Add a garment to the project, in its own fabric and colour, worn over the others (see garment.set_layer). Move pieces into it with panel.set_garment; template.apply on a project with pieces makes one by itself. Returns the garment's id.";
}

impl CommandSpec for GarmentAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(CommandError::invalid("name", Invalid::Empty));
        }
        check_color(self.color.as_deref())?;
        let layer = tx
            .project()
            .garments()
            .values()
            .map(|g| g.layer + 1)
            .max()
            .unwrap_or(0);
        let id = tx.new_garment_id();
        tx.put_garment(
            id,
            Garment {
                name: name.to_owned(),
                fabric: self.fabric,
                color: self.color.clone(),
                layer,
                template: None,
            },
        );
        Ok(CommandOutput {
            created: vec![AnyId::Garment(id)],
        })
    }
}

/// Rename a garment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentRename {
    pub garment: GarmentId,
    pub name: String,
}

impl Described for GarmentRename {
    const SUMMARY: &'static str = "Rename a garment.";
}

impl CommandSpec for GarmentRename {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(CommandError::invalid("name", Invalid::Empty));
        }
        let (id, g) = garment(tx, Some(self.garment))?;
        tx.put_garment(
            id,
            Garment {
                name: name.to_owned(),
                ..g
            },
        );
        Ok(CommandOutput::default())
    }
}

/// Delete a garment with its pieces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentRemove {
    pub garment: GarmentId,
    /// Must be `true`: deleting is destructive (still undoable).
    #[serde(default)]
    pub confirm: bool,
}

impl Described for GarmentRemove {
    const SUMMARY: &'static str = "Delete a garment and all its pieces (as panel.delete does each). The project keeps at least one garment. Destructive: requires \"confirm\": true.";
}

impl CommandSpec for GarmentRemove {
    const DESTRUCTIVE: bool = true;

    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (id, _) = garment(tx, Some(self.garment))?;
        if tx.project().garments().len() < 2 {
            return Err(CommandError::invalid("garment", Invalid::LastGarment));
        }
        let pieces: Vec<PanelId> = tx
            .project()
            .panels()
            .iter()
            .filter(|(_, p)| p.garment == id)
            .map(|(pid, _)| *pid)
            .collect();
        for panel in pieces {
            PanelDelete {
                panel,
                confirm: true,
            }
            .apply(tx)?;
        }
        tx.remove_garment(id);
        normalize_layers(tx);
        Ok(CommandOutput::default())
    }

    fn confirmed(&self) -> bool {
        self.confirm
    }
}

/// Number the garments' layers 0, 1, 2… in their current order.
fn normalize_layers(tx: &mut Transaction<'_>) {
    for (layer, id) in tx.project().garments_by_layer().into_iter().enumerate() {
        let Some(g) = tx.project().garments().get(&id).cloned() else {
            continue;
        };
        let layer = u32::try_from(layer).unwrap_or(u32::MAX);
        if g.layer != layer {
            tx.put_garment(id, Garment { layer, ..g });
        }
    }
}

/// Choose where a garment is worn among the others.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentSetLayer {
    pub garment: GarmentId,
    /// 0 next to the body; the garments from there on move up or down to
    /// make room. A shirt tucked into trousers is under them (a lower
    /// layer), a shirt worn out over them.
    pub layer: u32,
}

impl Described for GarmentSetLayer {
    const SUMMARY: &'static str = "Choose where a garment is worn among the others: layer 0 is next to the body, the others move to make room. The garment worn over another drapes over it: a shirt under trousers is tucked in, over them it is worn out.";
}

impl CommandSpec for GarmentSetLayer {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (id, _) = garment(tx, Some(self.garment))?;
        let mut order = tx.project().garments_by_layer();
        let last = order.len().saturating_sub(1);
        let to = usize::try_from(self.layer).unwrap_or(usize::MAX);
        if to > last {
            return Err(CommandError::invalid(
                "layer",
                Invalid::Range(0.0, last as f64),
            ));
        }
        order.retain(|g| *g != id);
        order.insert(to, id);
        for (layer, gid) in order.into_iter().enumerate() {
            let Some(g) = tx.project().garments().get(&gid).cloned() else {
                continue;
            };
            let layer = u32::try_from(layer).unwrap_or(u32::MAX);
            if g.layer != layer {
                tx.put_garment(gid, Garment { layer, ..g });
            }
        }
        Ok(CommandOutput::default())
    }
}

/// Move a piece to another garment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSetGarment {
    pub panel: PanelId,
    pub garment: GarmentId,
}

impl Described for PanelSetGarment {
    const SUMMARY: &'static str = "Move a piece to another garment; its linked mirror copies (and the piece it mirrors) move with it. The piece then drapes in that garment's fabric, colour and layer.";
}

impl CommandSpec for PanelSetGarment {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (garment_id, _) = garment(tx, Some(self.garment))?;
        let p = tx.project();
        let panel = p
            .panel(self.panel)
            .ok_or_else(|| CommandError::not_found(self.panel))?;
        // The mirror family: the source and every copy of it.
        let root = panel.mirror_of.as_ref().map_or(self.panel, |m| m.source);
        let family: Vec<PanelId> = p
            .panels()
            .iter()
            .filter(|(id, pn)| {
                **id == root || pn.mirror_of.as_ref().is_some_and(|m| m.source == root)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in family {
            let Some(pn) = tx.project().panel(id).cloned() else {
                continue;
            };
            if pn.garment != garment_id {
                tx.put_panel(
                    id,
                    r3d_core::Panel {
                        garment: garment_id,
                        ..pn
                    },
                );
            }
        }
        Ok(CommandOutput::default())
    }
}

fn check_color(color: Option<&str>) -> Result<(), CommandError> {
    if let Some(c) = color {
        let ok =
            c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit());
        if !ok {
            return Err(CommandError::invalid("color", Invalid::Format));
        }
    }
    Ok(())
}
