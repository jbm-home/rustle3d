//! Patterns and measurement files of Seamly2D and Valentina, two
//! open-source pattern programs whose XML formats are open (spec 28):
//! `.val` and `.sm2d` patterns, `.vit`/`.smis` individual and
//! `.vst`/`.smms` multisize measurements.
//!
//! A pattern there is a list of drafting steps (a point at a length and
//! an angle from another, the meeting of two lines, a curve between two
//! points…) whose lengths are formulas of body measurements. It is run
//! here for a set of measurement values ([`evaluate`]), which gives the
//! pieces at that body; run again for other bodies, it gives the sizes.
//! Implemented from the formats' description, no code of either program.

mod curve;
mod eval;
mod formula;
pub mod measure;
mod piece;

use std::collections::HashMap;

use r3d_core::{Body, Localizable, Message};

use super::{ImportedPattern, ImportedPiece, ScaleSource, finish, is_simple};
use eval::Draft;
pub use measure::{BODY_NAMES, FileSize, Measurements, Value, Values};
use piece::Pieces;

/// Why a Seamly2D/Valentina file could not be read or run.
#[derive(Clone, Debug, thiserror::Error, PartialEq)]
pub enum ValError {
    #[error("this pattern file cannot be read: {detail}")]
    Malformed { detail: String },
    #[error("unknown unit `{unit}`")]
    Unit { unit: String },
    #[error("the formula `{formula}` cannot be computed: {detail}")]
    Formula { formula: String, detail: String },
    #[error(
        "the formula uses `{name}`, which is not known (a measurement missing from the measurement file?)"
    )]
    UnknownName { name: String },
    #[error("object {id} is missing from the pattern")]
    MissingObject { id: u32 },
    #[error("drafting step {tool} ({id}) cannot be done: {detail}")]
    Construction {
        tool: String,
        id: u32,
        detail: String,
    },
    #[error("no piece of the pattern could be drafted")]
    NoPieces,
    #[error(
        "the pattern's measurement file {file} is not next to it in the project folder: copy it there, or name it with `measurements`"
    )]
    MissingMeasurementFile { file: String },
}

impl Localizable for ValError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Malformed { detail } => {
                Message::new("import_val_malformed", text).arg("detail", detail)
            }
            Self::Unit { unit } => Message::new("import_val_unit", text).arg("unit", unit),
            Self::Formula { formula, detail } => Message::new("import_val_formula", text)
                .arg("formula", formula)
                .arg("detail", detail),
            Self::UnknownName { name } => {
                Message::new("import_val_unknown_name", text).arg("name", name)
            }
            Self::MissingObject { id } => {
                Message::new("import_val_missing_object", text).arg("id", id)
            }
            Self::Construction { tool, id, detail } => {
                Message::new("import_val_construction", text)
                    .arg("tool", tool)
                    .arg("id", id)
                    .arg("detail", detail)
            }
            Self::NoPieces => Message::new("import_val_no_pieces", text),
            Self::MissingMeasurementFile { file } => {
                Message::new("import_val_missing_measurements", text).arg("file", file)
            }
        }
    }
}

/// Millimetres in one unit of these files.
fn mm_per_unit(unit: &str) -> Result<f64, ValError> {
    match unit.trim() {
        "mm" => Ok(1.0),
        "cm" => Ok(10.0),
        "inch" | "in" => Ok(25.4),
        u => Err(ValError::Unit { unit: u.to_owned() }),
    }
}

fn root<'a>(doc: &'a roxmltree::Document<'a>) -> roxmltree::Node<'a, 'a> {
    doc.root_element()
}

fn document(bytes: &[u8]) -> Result<roxmltree::Document<'_>, ValError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ValError::Malformed {
        detail: "not UTF-8 text".into(),
    })?;
    roxmltree::Document::parse(text.trim_start_matches('\u{feff}')).map_err(|e| {
        ValError::Malformed {
            detail: e.to_string(),
        }
    })
}

/// The measurement file a pattern names, relative to the pattern's folder
/// (as written: either slash), if any.
pub fn linked_measurements(pattern: &[u8]) -> Result<Option<String>, ValError> {
    let doc = document(pattern)?;
    let r = root(&doc);
    if r.tag_name().name() != "pattern" {
        return Err(ValError::Malformed {
            detail: "not a pattern file".into(),
        });
    }
    Ok(r.children()
        .find(|c| c.has_tag_name("measurements"))
        .and_then(|m| m.attribute("path").or_else(|| m.text()))
        .map(|p| p.trim().replace('\\', "/"))
        .filter(|p| !p.is_empty()))
}

/// The measurement values for a pattern: the file's, for `body` when one
/// is given; without a file, the body's height, bust, waist and hips.
pub fn values_for(file: Option<&Measurements>, body: Option<&Body>) -> Result<Values, ValError> {
    match file {
        Some(m) => m.values(body),
        None => Ok(body
            .map(|b| {
                BODY_NAMES
                    .iter()
                    .map(|(name, get)| {
                        (
                            (*name).to_owned(),
                            Value {
                                value: get(b) * 10.0,
                                length: true,
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()),
    }
}

/// A pattern run for one set of measurements.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluated {
    /// Each piece with its id in the file, in mm with y up, where the
    /// pattern draws it.
    pub pieces: Vec<(u32, ImportedPiece)>,
    /// Pieces that could not be built (a step of a kind not supported, an
    /// outline that crosses itself).
    pub ignored: usize,
    /// The kinds of steps not supported (`point:parallelCurve`).
    pub unsupported: Vec<String>,
}

/// Run a pattern with these measurement values.
pub fn evaluate(pattern: &[u8], values: &Values) -> Result<Evaluated, ValError> {
    let doc = document(pattern)?;
    let r = root(&doc);
    if r.tag_name().name() != "pattern" {
        return Err(ValError::Malformed {
            detail: "not a pattern file".into(),
        });
    }
    let unit = r
        .children()
        .find(|c| c.has_tag_name("unit"))
        .and_then(|u| u.text())
        .unwrap_or("cm");
    let mut draft = Draft::new(mm_per_unit(unit)?, values);
    for list in r.children().filter(|c| {
        c.has_tag_name("increments")
            || c.has_tag_name("previewCalculations")
            || c.has_tag_name("variables")
    }) {
        for v in list
            .children()
            .filter(|v| v.is_element() && v.attribute("type") != Some("separator"))
        {
            if let Some(name) = v.attribute("name") {
                draft.variable(name, v.attribute("formula").unwrap_or("0"))?;
            }
        }
    }
    let blocks: Vec<_> = r
        .children()
        .filter(|c| c.has_tag_name("draw") || c.has_tag_name("draftBlock"))
        .collect();
    for b in &blocks {
        if let Some(calc) = b.children().find(|c| c.has_tag_name("calculation")) {
            draft.calculation(calc)?;
        }
    }
    let mut modeling = HashMap::new();
    let mut paths = HashMap::new();
    let mut details = Vec::new();
    for b in &blocks {
        for m in b
            .children()
            .filter(|c| c.has_tag_name("modeling"))
            .flat_map(|m| m.children().filter(|c| c.is_element()))
        {
            let Some(id) = m.attribute("id").and_then(|v| v.parse::<u32>().ok()) else {
                continue;
            };
            if m.has_tag_name("path") {
                paths.insert(id, m);
            } else if let Some(obj) = m.attribute("idObject").and_then(|v| v.parse::<u32>().ok()) {
                modeling.insert(id, obj);
            }
        }
        details.extend(
            b.children()
                .filter(|c| c.has_tag_name("details") || c.has_tag_name("pieces"))
                .flat_map(|d| {
                    d.children()
                        .filter(|c| c.has_tag_name("detail") || c.has_tag_name("piece"))
                }),
        );
    }
    let pieces = Pieces {
        draft: &draft,
        modeling,
        paths,
    };
    let mut out = Vec::new();
    let mut ignored = 0;
    for d in details {
        if d.attribute("inLayout") == Some("false") {
            continue;
        }
        let id = d.attribute("id").and_then(|v| v.parse::<u32>().ok());
        match (id, pieces.piece(d)) {
            (Some(id), Some(p)) if is_simple(&p.outline) => out.push((id, p)),
            _ => ignored += 1,
        }
    }
    Ok(Evaluated {
        pieces: out,
        ignored,
        unsupported: draft.unsupported.iter().cloned().collect(),
    })
}

/// What [`read`] gives: the pieces laid out side by side, and for each
/// its id in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub pattern: ImportedPattern,
    pub ids: Vec<u32>,
    pub unsupported: Vec<String>,
}

/// Run a pattern with `values` and lay its pieces out side by side
/// (patterns often draw pieces over each other).
pub fn read(pattern: &[u8], values: &Values) -> Result<Read, ValError> {
    let e = evaluate(pattern, values)?;
    let ids = e.pieces.iter().map(|(id, _)| *id).collect();
    let raw = e
        .pieces
        .into_iter()
        .map(|(_, p)| {
            let o = p.outline.clone();
            (p, o)
        })
        .collect();
    Ok(Read {
        pattern: finish(raw, e.ignored, 1.0, ScaleSource::File).map_err(|_| ValError::NoPieces)?,
        ids,
        unsupported: e.unsupported,
    })
}

#[cfg(test)]
mod tests;
