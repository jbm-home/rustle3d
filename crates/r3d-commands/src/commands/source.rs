//! Parametric patterns from Seamly2D/Valentina (spec 28): importing one
//! as a garment that can be drafted again, drafting it again for a body,
//! and taking a body and sizes from a measurement file.

use r3d_core::{AnyId, Body, Garment, GarmentId, PatternSource, SourcePiece, Transaction};
use r3d_io::import::val::{self, BODY_NAMES, Measurements};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AvatarSet, GarmentAdd, GarmentRename, PanelSetGarment, PatternImport, PieceSpec, SizeSpec,
    VariableSet, set_sizes, validate_body,
};
use crate::error::{CommandError, Invalid, non_blank};
use crate::progress::Progress;
use crate::sources;
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Import a parametric pattern as a garment of its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatternImportParametric {
    /// The pattern file's name, as shown (`imports/skirt.val`).
    pub file: String,
    /// The pattern file's text: a Seamly2D (`.sm2d`) or Valentina
    /// (`.val`) pattern.
    pub pattern: String,
    /// The text of its measurement file (`.vit`, `.vst`, `.smis`,
    /// `.smms`), if it has one. Without it, the pattern is drafted for the
    /// avatar's height, bust, waist and hips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements: Option<String>,
}

impl Described for PatternImportParametric {
    const SUMMARY: &'static str = "Import a Seamly2D or Valentina pattern (the text of a .sm2d or .val file, and of its measurement file if it has one): its drafting steps are run with the measurements (without a measurement file, the avatar's height, bust, waist and hips) and its pieces added, with seam allowances, notches, grainlines and internal paths, laid out side by side to the right of the board. They make a garment of their own (the project's only garment on an empty board), which keeps the pattern: graded in every size of a size range, and drafted again for another body with garment.redraft. `import.pattern` reads the files. Returns the garment if one was added, then the panels.";
}

impl CommandSpec for PatternImportParametric {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    /// In a size range, the pattern is run for every size: reported size
    /// by size.
    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        non_blank("file", &self.file)?;
        // Without a measurement file, the pattern is drafted for the
        // avatar, and remembers it.
        let body = match self.measurements {
            Some(_) => None,
            None => tx.project().avatar().map(|a| a.body),
        };
        let mut source = PatternSource {
            file: self.file.trim().to_owned(),
            pattern: self.pattern.clone(),
            measurements: self.measurements.clone(),
            body,
            pieces: Vec::new(),
        };
        let values = sources::values(&source, body.as_ref())?;
        let read = val::read(self.pattern.as_bytes(), &values)?;
        let fresh = tx.project().panels().is_empty();
        let out = PatternImport {
            pieces: read
                .pattern
                .pieces
                .iter()
                .map(PieceSpec::from_imported)
                .collect(),
            keep_position: false,
        }
        .apply(tx)?;
        let panels: Vec<_> = out
            .created
            .iter()
            .filter_map(|id| match id {
                AnyId::Panel(p) => Some(*p),
                _ => None,
            })
            .collect();
        source.pieces = panels
            .iter()
            .zip(&read.ids)
            .map(|(panel, piece)| SourcePiece {
                panel: *panel,
                piece: *piece,
            })
            .collect();
        let name = std::path::Path::new(source.file.as_str())
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Pattern")
            .to_owned();
        let mut created = Vec::new();
        let garment = if fresh {
            let garment = tx
                .project()
                .default_garment()
                .ok_or(CommandError::Internal {
                    detail: "project has no garment".into(),
                })?;
            GarmentRename { garment, name }.apply(tx)?;
            garment
        } else {
            let added = GarmentAdd {
                name,
                fabric: Default::default(),
                color: None,
            }
            .apply(tx)?;
            let Some(AnyId::Garment(garment)) = added.created.first().copied() else {
                return Err(CommandError::Internal {
                    detail: "garment.add returned no garment".into(),
                });
            };
            for panel in &panels {
                PanelSetGarment {
                    panel: *panel,
                    garment,
                }
                .apply(tx)?;
            }
            created.push(AnyId::Garment(garment));
            garment
        };
        let g = tx
            .project()
            .garments()
            .get(&garment)
            .cloned()
            .ok_or_else(|| CommandError::not_found(garment))?;
        tx.put_garment(
            garment,
            Garment {
                source: Some(source),
                ..g
            },
        );
        sources::grade(tx, garment, progress)?;
        created.extend(out.created);
        Ok(CommandOutput { created })
    }
}

/// Draft an imported parametric pattern again for a body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentRedraft {
    pub garment: GarmentId,
    /// Measurements in cm; default: the avatar's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
}

impl Described for GarmentRedraft {
    const SUMMARY: &'static str = "Draft a garment imported from a Seamly2D or Valentina pattern again for a body (default: the avatar's): its pattern is run with that body's height, bust, waist and hips in place of the measurement file's (a multisize file is read at the size that fits it), and each point and curve moves as the pattern moves it, so changes made to the pieces since are kept; internal lines and pins follow. A piece whose outline gained or lost points is left as it is and listed in `skipped`. In a size range the garment is graded again.";
}

impl CommandSpec for GarmentRedraft {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        if !tx.project().garments().contains_key(&self.garment) {
            return Err(CommandError::not_found(self.garment));
        }
        let body = match self.body {
            Some(b) => {
                validate_body(&b)?;
                b
            }
            None => tx
                .project()
                .avatar()
                .map(|a| a.body)
                .ok_or(CommandError::invalid("body", Invalid::NoBody))?,
        };
        sources::redraft(tx, self.garment, body, progress)?;
        Ok(CommandOutput::default())
    }
}

/// Take the avatar's body (and sizes) from a measurement file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarUseMeasurements {
    /// The text of a Seamly2D or Valentina measurement file: individual
    /// (`.vit`, `.smis`) or multisize (`.vst`, `.smms`).
    pub measurements: String,
}

impl Described for AvatarUseMeasurements {
    const SUMMARY: &'static str = "Take the avatar's measurements from a Seamly2D or Valentina measurement file (its text): height, bust, waist and hips (height, bust_circ, waist_circ, hip_circ) become the body, the file's gender its masculinity; its other measurements become variables for formulas (in mm; a custom @name without the @). A multisize file also sets the size range: its sizes along the chest (or its first) dimension, its base size as the base. `import.measurements` reads the file.";
}

impl CommandSpec for AvatarUseMeasurements {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        let file = Measurements::parse(self.measurements.as_bytes())?;
        let values = file.values(None)?;
        let cm = |v: &val::Values, name: &str| {
            v.get(name)
                .filter(|v| v.length)
                .map(|v| v.value / 10.0)
                .ok_or(CommandError::invalid(
                    "measurements",
                    Invalid::MissingMeasurement,
                ))
        };
        let masculinity = file
            .masculinity()
            .or_else(|| tx.project().avatar().map(|a| a.body.masculinity))
            .unwrap_or(0.0);
        let body = Body {
            masculinity,
            height_cm: cm(&values, "height")?,
            bust_cm: cm(&values, "bust_circ")?,
            waist_cm: cm(&values, "waist_circ")?,
            hips_cm: cm(&values, "hip_circ")?,
        };
        AvatarSet { body, preset: None }.apply(tx)?;
        for (name, v) in &values {
            if BODY_NAMES.iter().any(|(n, _)| n == name) {
                continue;
            }
            let name = name.trim_start_matches('@');
            if !r3d_core::formula::valid_name(name) {
                continue;
            }
            let formula = if v.length {
                format!("{} mm", round(v.value))
            } else {
                format!("{}", round(v.value))
            };
            VariableSet {
                name: name.to_owned(),
                formula,
            }
            .apply(tx)?;
        }
        let sizes = file.sizes();
        if let Some(base) = sizes.iter().find(|s| s.base) {
            let specs = sizes
                .iter()
                .map(|s| {
                    let d = |name: &str| -> Result<f64, CommandError> {
                        Ok(cm(&s.values, name)? - cm(&base.values, name)?)
                    };
                    Ok(SizeSpec {
                        name: s.name.clone(),
                        height_cm: d("height")?,
                        bust_cm: d("bust_circ")?,
                        waist_cm: d("waist_circ")?,
                        hips_cm: d("hip_circ")?,
                    })
                })
                .collect::<Result<Vec<_>, CommandError>>()?;
            set_sizes(tx, &specs, &base.name, progress)?;
        }
        Ok(CommandOutput::default())
    }
}

/// A value rounded to a hundredth of a millimetre, as written.
fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The garments imported from a parametric pattern.
pub(crate) fn source_garments(tx: &Transaction<'_>) -> Vec<GarmentId> {
    tx.project()
        .garments()
        .iter()
        .filter(|(_, g)| g.source.is_some())
        .map(|(id, _)| *id)
        .collect()
}
