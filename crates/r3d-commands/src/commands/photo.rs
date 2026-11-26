//! How the project's photos are taken (spec 35).

use r3d_core::photo::{
    BRIGHTNESS, DISTANCE_MM, FOCAL_MM, PITCH_DEG, PLACE_SIZE_M, SIDE, SetupProblem,
};
use r3d_core::{PhotoSetup, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, Invalid};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Set how the project's photos are taken, or forget it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotoSetScene {
    /// The light, its turn and brightness, the camera and the image size;
    /// `null` forgets them (photos go back to their defaults).
    pub scene: Option<PhotoSetup>,
}

impl Described for PhotoSetScene {
    const SUMMARY: &'static str = "Set how the project's photos are taken: the light (a studio, or an environment by built-in name or `.hdr` path in the project), its turn and brightness, an environment's size (its radius around the model: the floor ends there and the rest of the picture stands around it), the camera (the point it looks at, its turn, height angle, distance and focal length) and the image size. The app's photo scene saves it; `render_photo` and `r3d render --view photo` use it unless their arguments say otherwise. `null` forgets it.";
}

/// The field and the error a refused setup reports.
fn refusal(problem: SetupProblem) -> CommandError {
    let range = |field: &'static str, r: std::ops::RangeInclusive<f64>| {
        CommandError::invalid(field, Invalid::Range(*r.start(), *r.end()))
    };
    match problem {
        SetupProblem::Environment => CommandError::invalid("scene.light", Invalid::Format),
        SetupProblem::Pixels => CommandError::invalid(
            "scene.size",
            Invalid::Range(1.0, r3d_core::photo::MAX_PIXELS as f64),
        ),
        SetupProblem::Range(field) => match field {
            "brightness" => range("scene.brightness", BRIGHTNESS),
            "place_size_m" => range("scene.place_size_m", PLACE_SIZE_M),
            "camera.pitch_deg" => range("scene.camera.pitch_deg", PITCH_DEG),
            "camera.distance_mm" => range("scene.camera.distance_mm", DISTANCE_MM),
            "camera.focal_mm" => range("scene.camera.focal_mm", FOCAL_MM),
            "size" => range(
                "scene.size",
                f64::from(*SIDE.start())..=f64::from(*SIDE.end()),
            ),
            "turn_deg" => CommandError::invalid("scene.turn_deg", Invalid::NotFinite),
            "camera.yaw_deg" => CommandError::invalid("scene.camera.yaw_deg", Invalid::NotFinite),
            _ => CommandError::invalid("scene.camera.target_mm", Invalid::NotFinite),
        },
    }
}

impl CommandSpec for PhotoSetScene {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if let Some(scene) = &self.scene {
            scene
                .check(&crate::environments::builtin_names())
                .map_err(refusal)?;
        }
        if tx.project().photo() != self.scene.as_ref() {
            tx.set_photo(self.scene.clone());
        }
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some("photo.set_scene".into())
    }
}
