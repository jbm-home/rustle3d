//! Body presets: built-in standard sizes and the user's own named presets.

use std::path::Path;

use serde::{Deserialize, Serialize};

use r3d_core::Body;

/// A named set of measurements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyPreset {
    pub name: String,
    pub body: Body,
}

const fn spec(masculinity: f64, height: f64, bust: f64, waist: f64, hips: f64) -> Body {
    Body {
        masculinity,
        height_cm: height,
        bust_cm: bust,
        waist_cm: waist,
        hips_cm: hips,
    }
}

/// Standard sizes. Approximate values from common European size charts
/// (general garment-making knowledge); good starting points to adjust.
pub fn builtin() -> Vec<BodyPreset> {
    let table: [(&str, Body); 9] = [
        ("Women XS", spec(0.0, 166.0, 80.0, 63.0, 88.0)),
        ("Women S", spec(0.0, 168.0, 84.0, 67.0, 92.0)),
        ("Women M", spec(0.0, 168.0, 90.0, 73.0, 98.0)),
        ("Women L", spec(0.0, 170.0, 98.0, 81.0, 106.0)),
        ("Women XL", spec(0.0, 170.0, 106.0, 89.0, 114.0)),
        ("Men S", spec(1.0, 176.0, 90.0, 78.0, 92.0)),
        ("Men M", spec(1.0, 178.0, 98.0, 86.0, 100.0)),
        ("Men L", spec(1.0, 180.0, 106.0, 94.0, 106.0)),
        ("Men XL", spec(1.0, 182.0, 114.0, 102.0, 112.0)),
    ];
    table
        .into_iter()
        .map(|(name, body)| BodyPreset {
            name: name.to_owned(),
            body,
        })
        .collect()
}

/// Errors reading or writing the user's preset file.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("cannot access {path}: {detail}")]
    Io { path: String, detail: String },
    #[error("invalid preset file {path}: {detail}")]
    Format { path: String, detail: String },
}

/// The user's own presets, stored as a JSON file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PresetLibrary {
    pub presets: Vec<BodyPreset>,
}

impl PresetLibrary {
    /// Load the library; a missing file is an empty library.
    pub fn load(path: &Path) -> Result<Self, LibraryError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(LibraryError::Io {
                    path: path.display().to_string(),
                    detail: e.to_string(),
                });
            }
        };
        serde_json::from_str(&text).map_err(|e| LibraryError::Format {
            path: path.display().to_string(),
            detail: e.to_string(),
        })
    }

    /// Save atomically (temporary file, then rename).
    pub fn save(&self, path: &Path) -> Result<(), LibraryError> {
        let io = |e: std::io::Error| LibraryError::Io {
            path: path.display().to_string(),
            detail: e.to_string(),
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| LibraryError::Format {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text + "\n").map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    /// Add a preset, replacing one with the same name.
    pub fn upsert(&mut self, preset: BodyPreset) {
        match self.presets.iter_mut().find(|p| p.name == preset.name) {
            Some(existing) => *existing = preset,
            None => self.presets.push(preset),
        }
    }

    /// Remove a preset by name; returns whether it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.presets.len();
        self.presets.retain(|p| p.name != name);
        self.presets.len() != before
    }
}
