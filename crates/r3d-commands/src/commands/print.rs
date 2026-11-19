//! A garment's print and how each piece places it (spec 25).

use glam::DVec2;
use r3d_core::print::{MAX_REPEAT_MM, MIN_REPEAT_MM, PrintPlacement, image_path_ok};
use r3d_core::{Garment, GarmentId, PanelId, Print, PrintSource, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Vec2;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// The garment `id`, or the project's when none is given.
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

/// An angle in (−180, 180].
fn normalized_deg(a: f64) -> f64 {
    let a = a.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// Put a print on the garment's fabric.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentSetPrint {
    /// Defaults to the project's garment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
    /// `{"builtin": "stripes" | "gingham" | "dots" | "twill"}`, or
    /// `{"image": "textures/name.png"}`: a PNG or JPEG file in the project
    /// folder (relative, forward slashes).
    pub print: PrintSource,
    /// Width of one repeat across the grain (mm, 1–5000); omitted: the
    /// built-in's usual repeat (20 mm stripes, 12 mm gingham, 16 mm dots,
    /// 4 mm twill), 100 mm for an image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    /// Height of one repeat along the grain (mm, 1–5000); omitted: the
    /// image's proportions (square for built-ins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    /// The print turned from the grainline (degrees, counter-clockwise);
    /// 0: the image's top along the grain.
    #[serde(default)]
    pub angle_deg: f64,
}

/// Repeat of an image when none is given (mm).
const IMAGE_REPEAT_MM: f64 = 100.0;

impl Described for GarmentSetPrint {
    const SUMMARY: &'static str = "Put a print on the garment's fabric: a built-in one (stripes, gingham, dots, twill, drawn in the garment's colour) or a PNG/JPEG image in the project folder (`r3d texture` copies one in). The print lies along each piece's grainline at its real size (`width_mm` × `height_mm` per repeat), turned by `angle_deg`; it shows in the 3D view, photos, the board and the glTF/OBJ exports. Replaces the previous print.";
}

impl CommandSpec for GarmentSetPrint {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let repeat = |field: &'static str, v: f64| {
            finite(field, &[v])?;
            if !(MIN_REPEAT_MM..=MAX_REPEAT_MM).contains(&v) {
                return Err(CommandError::invalid(
                    field,
                    Invalid::Range(MIN_REPEAT_MM, MAX_REPEAT_MM),
                ));
            }
            Ok(v)
        };
        let width_mm = match (self.width_mm, &self.print) {
            (Some(w), _) => repeat("width_mm", w)?,
            (None, PrintSource::Builtin(b)) => b.default_repeat_mm(),
            (None, PrintSource::Image(_)) => IMAGE_REPEAT_MM,
        };
        let height_mm = self.height_mm.map(|h| repeat("height_mm", h)).transpose()?;
        finite("angle_deg", &[self.angle_deg])?;
        if let PrintSource::Image(path) = &self.print
            && !image_path_ok(path)
        {
            return Err(CommandError::invalid("print", Invalid::Format));
        }
        let (id, g) = garment(tx, self.garment)?;
        tx.put_garment(
            id,
            Garment {
                print: Some(Print {
                    source: self.print.clone(),
                    width_mm,
                    height_mm,
                    angle_deg: normalized_deg(self.angle_deg),
                }),
                ..g
            },
        );
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!(
            "garment.set_print:{}",
            self.garment.map(|g| g.to_string()).unwrap_or_default()
        ))
    }
}

/// Take the print off the garment's fabric.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentRemovePrint {
    /// Defaults to the project's garment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
}

impl Described for GarmentRemovePrint {
    const SUMMARY: &'static str = "Take the print off the garment: back to its plain colour. The image file, if any, stays in the project folder.";
}

impl CommandSpec for GarmentRemovePrint {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (id, g) = garment(tx, self.garment)?;
        if g.print.is_some() {
            tx.put_garment(id, Garment { print: None, ..g });
        }
        Ok(CommandOutput::default())
    }
}

/// Shift and turn the garment's print on one piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSetPrintPlacement {
    pub panel: PanelId,
    /// The print moved on the piece (mm, board axes); default none.
    #[serde(default)]
    pub offset_mm: Vec2,
    /// The print turned on the piece (degrees, counter-clockwise),
    /// added to the garment's angle; default 0.
    #[serde(default)]
    pub angle_deg: f64,
}

impl Described for PanelSetPrintPlacement {
    const SUMMARY: &'static str = "Shift (mm) and turn (degrees) the garment's print on one piece, e.g. to centre a motif on a front or match stripes across a seam. Zero for both lays the print as on every other piece.";
}

impl CommandSpec for PanelSetPrintPlacement {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("offset_mm", &self.offset_mm)?;
        finite("angle_deg", &[self.angle_deg])?;
        let mut panel = tx
            .project()
            .panel(self.panel)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.panel))?;
        let print = PrintPlacement {
            offset_mm: DVec2::from_array(self.offset_mm),
            angle_deg: normalized_deg(self.angle_deg),
        };
        if panel.print != print {
            panel.print = print;
            tx.put_panel(self.panel, panel);
        }
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.set_print_placement:{}", self.panel))
    }
}

#[cfg(test)]
mod tests {
    use super::normalized_deg;

    #[test]
    fn angles_are_kept_in_a_half_turn_each_way() {
        assert_eq!(normalized_deg(190.0), -170.0);
        assert_eq!(normalized_deg(-180.0), 180.0);
        assert_eq!(normalized_deg(45.0), 45.0);
    }
}
