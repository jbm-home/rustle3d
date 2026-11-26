//! How the project's photos are taken (spec 35): the light, the camera
//! and the image size, set in the app's photo scene and used again by the
//! command line and agents.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The brightness factor's range.
pub const BRIGHTNESS: std::ops::RangeInclusive<f64> = 0.05..=20.0;
/// How far an environment's walls stand from the model at floor level (m).
pub const PLACE_SIZE_M: std::ops::RangeInclusive<f64> = 1.0..=100.0;
/// The focal length's range (mm, on a 24 × 36 mm frame).
pub const FOCAL_MM: std::ops::RangeInclusive<f64> = 10.0..=300.0;
/// The camera's distance from the point it looks at (mm).
pub const DISTANCE_MM: std::ops::RangeInclusive<f64> = 300.0..=20_000.0;
/// The camera's height angle (degrees; up to 80° above or below).
pub const PITCH_DEG: std::ops::RangeInclusive<f64> = -80.0..=80.0;
/// An image side (pixels).
pub const SIDE: std::ops::RangeInclusive<u32> = 16..=4096;
/// Most pixels in a photo (3840 × 2160 fits).
pub const MAX_PIXELS: u64 = 1 << 23;
/// Half the shorter side of a 24 × 36 mm frame (mm).
const HALF_FRAME_MM: f64 = 12.0;

/// What lights a photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhotoLight {
    /// A key, a fill and a rim light on a light grey backdrop.
    Studio,
    /// Large soft boxes on a white backdrop, few shadows.
    Soft,
    /// One hard key and a rim on a dark backdrop.
    Dramatic,
    /// A picture of the light all around: a built-in environment's name
    /// (`ferndale`, `neon`, `pav`, `kontrast`, `wooden`) or a `.hdr` file
    /// in the project folder (relative, forward slashes).
    Environment { name: String },
}

/// Where the photo's camera stands: it turns around the point it looks
/// at. Metres in the simulation are millimetres here (Y up).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotoCamera {
    /// The point looked at (mm).
    pub target_mm: [f64; 3],
    /// Turn around the vertical (degrees; 0: from the front, 90: from the
    /// body's left side).
    pub yaw_deg: f64,
    /// Height angle (degrees, −80 to 80; positive: from above).
    pub pitch_deg: f64,
    /// Distance from the point looked at (mm, 300–20 000).
    pub distance_mm: f64,
    /// Focal length on a 24 × 36 mm frame (mm, 10–300): its 24 mm side
    /// lies along the image's shorter side.
    pub focal_mm: f64,
}

impl PhotoCamera {
    /// The field of view along the image's shorter side (radians).
    pub fn field_of_view(&self) -> f64 {
        2.0 * (HALF_FRAME_MM / self.focal_mm).atan()
    }

    /// The focal length that gives `fov` along the shorter side.
    pub fn focal_for(fov: f64) -> f64 {
        HALF_FRAME_MM / (fov / 2.0).tan()
    }
}

/// How the project's photos are taken.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotoSetup {
    pub light: PhotoLight,
    /// Turn of the light around the model (degrees, counter-clockwise
    /// seen from above); at 0 an environment's centre is behind the model
    /// and the studio's key light on the camera's left.
    #[serde(default)]
    pub turn_deg: f64,
    /// Factor on the light (0.05–20; 1: the usual exposure).
    #[serde(default = "one")]
    pub brightness: f64,
    /// How far an environment's walls stand from the model at floor
    /// level (m, 1–100; 3: a room): its floor ends there and the rest of
    /// its picture stands around it.
    #[serde(default = "room")]
    pub place_size_m: f64,
    pub camera: PhotoCamera,
    /// Image size in pixels (16–4096 a side); omitted: the 3D view's size
    /// in the app, the command's own elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[u32; 2]>,
}

fn one() -> f64 {
    1.0
}

fn room() -> f64 {
    3.0
}

/// Why a photo setup is refused: the field and what is wrong with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupProblem {
    /// Not a finite number, or outside the field's range.
    Range(&'static str),
    /// Not a built-in environment's name nor a `.hdr` path in the project.
    Environment,
    /// Too many pixels.
    Pixels,
}

/// A path of a `.hdr` file inside the project folder: relative, forward
/// slashes, no `..`.
pub fn environment_path_ok(path: &str) -> bool {
    let p = std::path::Path::new(path);
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.starts_with('/')
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
        && p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("hdr"))
}

impl PhotoSetup {
    /// Check every number's range and the environment's name (one of
    /// `builtin`, or a path from [`environment_path_ok`]).
    pub fn check(&self, builtin: &[&str]) -> Result<(), SetupProblem> {
        if let PhotoLight::Environment { name } = &self.light
            && !builtin.contains(&name.as_str())
            && !environment_path_ok(name)
        {
            return Err(SetupProblem::Environment);
        }
        self.check_numbers()
    }

    /// Check every number's range (what a file must hold to be opened).
    pub fn check_numbers(&self) -> Result<(), SetupProblem> {
        let within = |field, v: f64, range: std::ops::RangeInclusive<f64>| {
            if v.is_finite() && range.contains(&v) {
                Ok(())
            } else {
                Err(SetupProblem::Range(field))
            }
        };
        within("turn_deg", self.turn_deg, -360_000.0..=360_000.0)?;
        within("brightness", self.brightness, BRIGHTNESS)?;
        within("place_size_m", self.place_size_m, PLACE_SIZE_M)?;
        let c = &self.camera;
        for v in c.target_mm {
            within("camera.target_mm", v, -100_000.0..=100_000.0)?;
        }
        within("camera.yaw_deg", c.yaw_deg, -360_000.0..=360_000.0)?;
        within("camera.pitch_deg", c.pitch_deg, PITCH_DEG)?;
        within("camera.distance_mm", c.distance_mm, DISTANCE_MM)?;
        within("camera.focal_mm", c.focal_mm, FOCAL_MM)?;
        if let Some([w, h]) = self.size {
            if !SIDE.contains(&w) || !SIDE.contains(&h) {
                return Err(SetupProblem::Range("size"));
            }
            if u64::from(w) * u64::from(h) > MAX_PIXELS {
                return Err(SetupProblem::Pixels);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> PhotoSetup {
        PhotoSetup {
            light: PhotoLight::Studio,
            turn_deg: 0.0,
            brightness: 1.0,
            place_size_m: 3.0,
            camera: PhotoCamera {
                target_mm: [0.0, 900.0, 0.0],
                yaw_deg: 30.0,
                pitch_deg: 5.0,
                distance_mm: 3600.0,
                focal_mm: 50.0,
            },
            size: None,
        }
    }

    #[test]
    fn focal_and_field_of_view_agree() {
        let c = setup().camera;
        let fov = c.field_of_view();
        assert!((PhotoCamera::focal_for(fov) - 50.0).abs() < 1e-9);
        // A 50 mm lens sees about 27° along the 24 mm side.
        assert!((fov.to_degrees() - 26.99).abs() < 0.01);
    }

    #[test]
    fn checks_ranges_and_names() {
        assert_eq!(setup().check(&["pav"]), Ok(()));
        let env = |name: &str| PhotoSetup {
            light: PhotoLight::Environment { name: name.into() },
            ..setup()
        };
        assert_eq!(env("pav").check(&["pav"]), Ok(()));
        assert_eq!(env("environments/a.HDR").check(&[]), Ok(()));
        for bad in ["nope", "../a.hdr", "/a.hdr", "a.png", "c:/a.hdr", ""] {
            assert_eq!(env(bad).check(&["pav"]), Err(SetupProblem::Environment));
        }
        let mut s = setup();
        s.camera.focal_mm = 5.0;
        assert_eq!(s.check(&[]), Err(SetupProblem::Range("camera.focal_mm")));
        let mut s = setup();
        s.brightness = f64::NAN;
        assert_eq!(s.check(&[]), Err(SetupProblem::Range("brightness")));
        let mut s = setup();
        s.place_size_m = 0.5;
        assert_eq!(s.check(&[]), Err(SetupProblem::Range("place_size_m")));
        let mut s = setup();
        s.size = Some([4096, 4096]);
        assert_eq!(s.check(&[]), Err(SetupProblem::Pixels));
        s.size = Some([8, 100]);
        assert_eq!(s.check(&[]), Err(SetupProblem::Range("size")));
    }
}
