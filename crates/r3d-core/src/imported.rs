//! A body imported from a file (spec 27): where the file is, how to turn
//! its coordinates into the avatar's frame, and its arrangement points.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A body mesh imported from an OBJ, glTF or GLB file inside the project
/// folder, draped instead of the built-in body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarMesh {
    /// Relative path inside the project folder, forward slashes
    /// (`avatars/scan.glb`).
    pub file: String,
    /// The file's content fingerprint (FNV-1a 64, 16 hex digits): a file
    /// changed by hand is noticed.
    pub hash: String,
    /// Metres per file unit.
    pub scale: f64,
    /// The file's axis that points up.
    pub up: UpAxis,
    /// Turn about the vertical (degrees, counter-clockwise seen from
    /// above) after the up axis, so that the body faces +Z.
    pub turn_deg: f64,
    /// The arrangement points (mm, in the avatar's frame: Y up, facing
    /// +Z, the body's left at +x, feet at y = 0).
    pub points: BTreeMap<BodyPoint, [f64; 3]>,
}

/// An axis of a model file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum UpAxis {
    #[serde(rename = "y")]
    Y,
    #[serde(rename = "z")]
    Z,
    #[serde(rename = "x")]
    X,
    #[serde(rename = "-y")]
    NegY,
    #[serde(rename = "-z")]
    NegZ,
    #[serde(rename = "-x")]
    NegX,
}

/// Points that tell the drape where the body's parts are (spec 27).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BodyPoint {
    /// The middle of the torso at the bust or chest girth.
    Chest,
    /// The middle of the torso at the waist girth.
    Waist,
    /// The middle of the torso at the hips girth.
    Hips,
    /// Where the legs part.
    Crotch,
    LeftShoulder,
    LeftElbow,
    LeftWrist,
    RightShoulder,
    RightElbow,
    RightWrist,
    /// The middle of the left thigh: the top of the left leg's axis.
    LeftThigh,
    /// The middle of the left shin: the axis goes through it.
    LeftShin,
    RightThigh,
    RightShin,
}

impl BodyPoint {
    pub const ALL: [BodyPoint; 14] = [
        Self::Chest,
        Self::Waist,
        Self::Hips,
        Self::Crotch,
        Self::LeftShoulder,
        Self::LeftElbow,
        Self::LeftWrist,
        Self::RightShoulder,
        Self::RightElbow,
        Self::RightWrist,
        Self::LeftThigh,
        Self::LeftShin,
        Self::RightThigh,
        Self::RightShin,
    ];

    /// +1 for a point of the body's left, −1 of its right, 0 on the
    /// middle.
    pub fn side(self) -> f64 {
        match self {
            Self::LeftShoulder
            | Self::LeftElbow
            | Self::LeftWrist
            | Self::LeftThigh
            | Self::LeftShin => 1.0,
            Self::RightShoulder
            | Self::RightElbow
            | Self::RightWrist
            | Self::RightThigh
            | Self::RightShin => -1.0,
            _ => 0.0,
        }
    }

    /// Its name as in the file (`left_elbow`).
    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

/// Whether `path` can name an imported body: relative, forward slashes,
/// inside the folder, an `.obj`, `.gltf` or `.glb` file.
pub fn model_path_ok(path: &str) -> bool {
    let p = std::path::Path::new(path);
    let inside = !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.starts_with('/')
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    inside && matches!(ext.as_deref(), Some("obj" | "gltf" | "glb"))
}

impl AvatarMesh {
    /// Every value is usable: a valid path and hash, a positive scale,
    /// finite numbers.
    pub fn valid(&self) -> bool {
        model_path_ok(&self.file)
            && self.hash.len() == 16
            && self.hash.chars().all(|c| c.is_ascii_hexdigit())
            && self.scale.is_finite()
            && self.scale > 0.0
            && self.turn_deg.is_finite()
            && self.points.values().flatten().all(|v| v.is_finite())
    }
}

/// The FNV-1a 64 fingerprint of a file, as stored in [`AvatarMesh::hash`].
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_paths_and_fingerprints() {
        assert_eq!(BodyPoint::LeftElbow.name(), "left_elbow");
        assert_eq!(BodyPoint::RightShin.side(), -1.0);
        assert!(model_path_ok("avatars/scan.GLB"));
        for bad in ["../x.obj", "/a.obj", "avatars\\a.obj", "a.png", ""] {
            assert!(!model_path_ok(bad), "{bad}");
        }
        assert_eq!(fingerprint(b""), "cbf29ce484222325");
        assert_eq!(fingerprint(b"a").len(), 16);
    }
}
