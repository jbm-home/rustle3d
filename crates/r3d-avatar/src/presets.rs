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
        age_years: None,
    }
}

/// Children's sizes by age: (age in years, height, chest, waist, hips in
/// cm). Approximate values from common European children's size charts
/// (general garment-making knowledge): the size is the age, the height
/// grows 6 cm a year up to 6, then 12 every two years.
const GIRLS: [(f64, f64, f64, f64, f64); 9] = [
    (2.0, 92.0, 53.0, 51.0, 55.0),
    (3.0, 98.0, 54.0, 52.0, 57.0),
    (4.0, 104.0, 56.0, 53.0, 59.0),
    (5.0, 110.0, 58.0, 54.0, 62.0),
    (6.0, 116.0, 60.0, 55.0, 64.0),
    (8.0, 128.0, 65.0, 58.0, 69.0),
    (10.0, 140.0, 70.0, 61.0, 75.0),
    (12.0, 152.0, 77.0, 63.0, 82.0),
    (14.0, 164.0, 83.0, 65.0, 89.0),
];
const BOYS: [(f64, f64, f64, f64, f64); 9] = [
    (2.0, 92.0, 53.0, 51.0, 54.0),
    (3.0, 98.0, 55.0, 52.0, 56.0),
    (4.0, 104.0, 57.0, 53.0, 58.0),
    (5.0, 110.0, 59.0, 54.0, 60.0),
    (6.0, 116.0, 61.0, 55.0, 62.0),
    (8.0, 128.0, 66.0, 58.0, 67.0),
    (10.0, 140.0, 71.0, 62.0, 72.0),
    (12.0, 152.0, 76.0, 65.0, 78.0),
    (14.0, 164.0, 82.0, 68.0, 84.0),
];
/// Babies' sizes by age in months, the same for girls and boys.
const BABIES: [(u32, f64, f64, f64, f64); 5] = [
    (3, 62.0, 42.0, 42.0, 43.0),
    (6, 68.0, 44.0, 44.0, 46.0),
    (12, 76.0, 47.0, 46.0, 48.0),
    (18, 82.0, 49.0, 47.0, 50.0),
    (24, 88.0, 51.0, 49.0, 52.0),
];

/// Who a children's chart dresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Children {
    Girls,
    Boys,
    Babies,
}

impl Children {
    /// Each size's name (the age: "4"; for babies the months: "12m") and
    /// body, youngest first.
    pub fn sizes(self) -> Vec<(String, Body)> {
        let child = |masculinity: f64, (age, h, c, w, hp): (f64, f64, f64, f64, f64)| {
            (
                age.to_string(),
                Body {
                    age_years: Some(age),
                    ..spec(masculinity, h, c, w, hp)
                },
            )
        };
        match self {
            Children::Girls => GIRLS.into_iter().map(|s| child(0.0, s)).collect(),
            Children::Boys => BOYS.into_iter().map(|s| child(1.0, s)).collect(),
            Children::Babies => BABIES
                .into_iter()
                .map(|(months, h, c, w, hp)| {
                    (
                        format!("{months}m"),
                        Body {
                            age_years: Some(f64::from(months) / 12.0),
                            ..spec(0.5, h, c, w, hp)
                        },
                    )
                })
                .collect(),
        }
    }

    /// The preset names' first word.
    fn preset_prefix(self) -> &'static str {
        match self {
            Children::Girls => "Girls",
            Children::Boys => "Boys",
            Children::Babies => "Baby",
        }
    }
}

/// Standard sizes, adults' then children's ([`Children`]). Approximate
/// values from common European size charts (general garment-making
/// knowledge); good starting points to adjust.
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
    let adults = table.into_iter().map(|(name, body)| BodyPreset {
        name: name.to_owned(),
        body,
    });
    let children = [Children::Babies, Children::Girls, Children::Boys]
        .into_iter()
        .flat_map(|c| {
            c.sizes().into_iter().map(move |(size, body)| BodyPreset {
                name: format!("{} {size}", c.preset_prefix()),
                body,
            })
        });
    adults.chain(children).collect()
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
