use std::collections::BTreeMap;

use r3d_core::{Avatar, AvatarMesh, Body, BodyPoint, Measure, Transaction, UpAxis};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, Invalid, finite, non_blank, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Accepted ranges (cm), generous enough for adults of any build and
/// babies.
pub const HEIGHT_RANGE: (f64, f64) = (50.0, 220.0);
pub const BUST_RANGE: (f64, f64) = (35.0, 170.0);
pub const WAIST_RANGE: (f64, f64) = (33.0, 170.0);
pub const HIPS_RANGE: (f64, f64) = (35.0, 180.0);

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
    in_range("hips_cm", b.hips_cm, HIPS_RANGE)?;
    match b.age_years {
        Some(age) => in_range("age_years", age, Body::AGE_RANGE),
        None => Ok(()),
    }
}

/// Set the avatar's measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarSet {
    /// The measurements: `masculinity` (0 = feminine body, 1 =
    /// masculine), `height_cm`, `bust_cm`, `waist_cm`, `hips_cm`, and
    /// `age_years` for a child.
    pub body: Body,
    /// Name of the preset the measurements come from, if any (a label:
    /// the body is still given in `body`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

impl Described for AvatarSet {
    const SUMMARY: &'static str = "Set the avatar's body. The measurements go inside `body`, not at the top level: {\"body\": {\"masculinity\": 0, \"height_cm\": 168, \"bust_cm\": 92, \"waist_cm\": 74, \"hips_cm\": 100}}; masculinity 0 feminine – 1 masculine, the rest in cm, and for a child `age_years` (0–18; left out: an adult). `preset` only records which preset the body came from: to use one, copy its `body` from avatar.presets. The 3D body is fitted to these measurements, with a child's proportions at that age; avatar.get reports what it actually measures.";
}

impl CommandSpec for AvatarSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        super::object::wearable(tx, "body")?;
        validate_body(&self.body)?;
        if let Some(p) = &self.preset {
            non_blank("preset", p)?;
        }
        // An imported body stays: these measurements only size templates
        // and formulas then.
        // The measurements typed besides the four stay too.
        let current = tx.project().avatar();
        let mesh = current.and_then(|a| a.mesh.clone());
        let measures = current.map(|a| a.measures.clone()).unwrap_or_default();
        tx.set_avatar(Some(Avatar {
            body: self.body,
            preset: self.preset.as_deref().map(str::trim).map(str::to_owned),
            mesh,
            measures,
        }));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("avatar.set".into())
    }
}

/// Set or reset the avatar's other measurements (spec 29).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarSetMeasures {
    /// By name (`neck`, `arm_length`…): the value in cm, or null to go
    /// back to the estimate.
    pub measures: BTreeMap<Measure, Option<f64>>,
}

impl Described for AvatarSetMeasures {
    const SUMMARY: &'static str = "Set the avatar's other measurements in cm by name: neck, shoulder_width, shoulder_length, back_waist_length, front_waist_length, waist_height, hip_height, crotch_depth (torso); arm_length, upper_arm, wrist (arms); inside_leg, thigh, knee, ankle (legs). Null goes back to the estimate from height, bust, waist and hips. The 3D body follows the ones it can (avatar.get says which and what it reaches); formulas and templates use them all, each size of a range graded like its estimate.";
}

impl CommandSpec for AvatarSetMeasures {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if self.measures.is_empty() {
            return Err(CommandError::invalid("measures", Invalid::Empty));
        }
        let mut avatar = tx
            .project()
            .avatar()
            .cloned()
            .ok_or_else(|| CommandError::invalid("measures", Invalid::NoBody))?;
        for (m, value) in &self.measures {
            match value {
                Some(cm) => {
                    in_range(m.name(), *cm, m.range_cm())?;
                    avatar.measures.insert(*m, *cm);
                }
                None => {
                    avatar.measures.remove(m);
                }
            }
        }
        tx.set_avatar(Some(avatar));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        let names: Vec<&str> = self.measures.keys().map(|m| m.name()).collect();
        Some(format!("avatar.set_measures:{}", names.join(",")))
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

/// The orientation of an imported body: given values, the others as
/// they are (or detected, for a new body).
fn orientation(
    base: crate::bodies::Orientation,
    scale: Option<f64>,
    up: Option<UpAxis>,
    turn_deg: Option<f64>,
) -> Result<crate::bodies::Orientation, CommandError> {
    if let Some(s) = scale {
        finite("scale", &[s])?;
        positive("scale", s)?;
    }
    if let Some(t) = turn_deg {
        finite("turn_deg", &[t])?;
    }
    Ok(crate::bodies::Orientation {
        scale: scale.unwrap_or(base.scale),
        up: up.unwrap_or(base.up),
        turn_deg: turn_deg.unwrap_or(base.turn_deg),
    })
}

/// Set the avatar to `mesh`, guessing its points and measuring it.
fn use_mesh(tx: &mut Transaction<'_>, mut mesh: AvatarMesh) -> Result<(), CommandError> {
    super::object::wearable(tx, "file")?;
    let placed = crate::bodies::placed(&mesh)?;
    mesh.points = crate::bodies::guess_points(&placed);
    let (body, _) = crate::bodies::measure(&placed, &mesh.points);
    let measures = tx
        .project()
        .avatar()
        .map(|a| a.measures.clone())
        .unwrap_or_default();
    tx.set_avatar(Some(Avatar {
        body,
        preset: None,
        mesh: Some(mesh),
        measures,
    }));
    Ok(())
}

/// The avatar and its imported body, refused when it has none.
fn imported(tx: &Transaction<'_>) -> Result<(Avatar, AvatarMesh), CommandError> {
    let avatar = tx.project().avatar().cloned();
    let mesh = avatar.as_ref().and_then(|a| a.mesh.clone());
    avatar.zip(mesh).ok_or(CommandError::NoImportedBody)
}

/// Drape on a body imported from a file already read (`avatar.import`
/// reads it; an agent calls that action rather than this command).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarImportMesh {
    /// The file, relative to the project folder (`avatars/scan.glb`).
    pub file: String,
    /// Its fingerprint, as `avatar.import` returns it.
    pub hash: String,
    /// Metres per file unit; detected when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// The file's up axis; detected when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<UpAxis>,
    /// Turn about the vertical (degrees) so the body faces +Z; detected
    /// when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_deg: Option<f64>,
}

impl Described for AvatarImportMesh {
    const SUMMARY: &'static str = "Drape on a body read from a file in the project folder (run `avatar.import`, which reads the file and calls this). Units, up axis and facing are detected unless given; the arrangement points are guessed and the measurements taken on the body (templates are sized from them).";
}

impl CommandSpec for AvatarImportMesh {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if !r3d_core::imported::model_path_ok(&self.file) {
            return Err(CommandError::invalid("file", Invalid::Format));
        }
        let loaded =
            crate::bodies::loaded(&self.hash).ok_or_else(|| CommandError::BodyUnavailable {
                file: self.file.clone(),
            })?;
        let o = orientation(
            crate::bodies::detect(&loaded),
            self.scale,
            self.up,
            self.turn_deg,
        )?;
        use_mesh(
            tx,
            AvatarMesh {
                file: self.file.clone(),
                hash: self.hash.clone(),
                scale: o.scale,
                up: o.up,
                turn_deg: o.turn_deg,
                points: Default::default(),
            },
        )?;
        Ok(CommandOutput::default())
    }
}

/// Correct the units, up axis or facing of the imported body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarOrient {
    /// Metres per file unit (0.01 for centimetres, 0.001 for millimetres,
    /// 0.0254 for inches).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// The file's axis that points up: `y`, `z`, `x`, `-y`, `-z`, `-x`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<UpAxis>,
    /// Turn about the vertical (degrees) so the body faces +Z.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_deg: Option<f64>,
}

impl Described for AvatarOrient {
    const SUMMARY: &'static str = "Correct how the imported body's file is read: its units (scale, metres per unit), which axis is up, and a turn about the vertical so it faces +Z. Omitted values stay. The arrangement points are guessed again and the measurements taken again.";
}

impl CommandSpec for AvatarOrient {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (_, mesh) = imported(tx)?;
        let o = orientation(
            crate::bodies::Orientation::of(&mesh),
            self.scale,
            self.up,
            self.turn_deg,
        )?;
        use_mesh(
            tx,
            AvatarMesh {
                scale: o.scale,
                up: o.up,
                turn_deg: o.turn_deg,
                ..mesh
            },
        )?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("avatar.orient".into())
    }
}

/// Move one arrangement point of the imported body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarSetPoint {
    pub point: BodyPoint,
    /// Where (mm, Y up, the body facing +Z, its left at +x, feet at 0).
    pub at_mm: [f64; 3],
}

impl Described for AvatarSetPoint {
    const SUMMARY: &'static str = "Move an arrangement point of the imported body (mm, Y up, facing +Z, the body's left at +x): chest, waist, hips (the measurements are taken again at their height), crotch, left/right_shoulder, _elbow, _wrist (the axis sleeves wrap around), left/right_thigh, _shin (the trouser legs' axis). A point must lie inside the body; avatar.check flags it otherwise.";
}

impl CommandSpec for AvatarSetPoint {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("at_mm", &self.at_mm)?;
        let (avatar, mut mesh) = imported(tx)?;
        mesh.points.insert(self.point, self.at_mm);
        let body = if matches!(
            self.point,
            BodyPoint::Chest | BodyPoint::Waist | BodyPoint::Hips
        ) {
            crate::bodies::measure(&*crate::bodies::placed(&mesh)?, &mesh.points).0
        } else {
            avatar.body
        };
        tx.set_avatar(Some(Avatar {
            body,
            mesh: Some(mesh),
            ..avatar
        }));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("avatar.set_point:{}", self.point.name()))
    }
}

/// Guess the imported body's arrangement points again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarGuessPoints {}

impl Described for AvatarGuessPoints {
    const SUMMARY: &'static str = "Guess the imported body's arrangement points again from its sections (arms down or in an A pose), and take its measurements again.";
}

impl CommandSpec for AvatarGuessPoints {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (_, mesh) = imported(tx)?;
        use_mesh(tx, mesh)?;
        Ok(CommandOutput::default())
    }
}

/// Go back to the built-in body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarRemoveMesh {}

impl Described for AvatarRemoveMesh {
    const SUMMARY: &'static str = "Drape on the built-in body again, fitted to the avatar's measurements (those taken on the imported body). The file stays in the project's avatars/ folder.";
}

impl CommandSpec for AvatarRemoveMesh {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (avatar, _) = imported(tx)?;
        tx.set_avatar(Some(Avatar {
            mesh: None,
            ..avatar
        }));
        Ok(CommandOutput::default())
    }
}
