//! Measurement files of Seamly2D/Valentina: individual (`.vit`, `.smis`:
//! one value per measurement, itself a formula) and multisize (`.vst`,
//! `.smms`: a base value and a shift per step of each dimension: height,
//! chest, waist, hips).
//!
//! The values for a body of ours come from the file: an individual file
//! takes the body's height, bust, waist and hips in place of its own (the
//! measurements computed from them follow); a multisize file is read at
//! the dimensions that fit the body best (least squares over those four
//! measurements, the file's model being linear in each dimension).

use std::collections::BTreeMap;

use r3d_core::Body;

use super::{ValError, formula, mm_per_unit, root};

/// Reads one measurement of a body (cm).
pub type BodyMeasure = fn(&Body) -> f64;

/// The measurements our bodies have, by their name in these files, and
/// how to read them from a [`Body`] (cm).
pub const BODY_NAMES: [(&str, BodyMeasure); 4] = [
    ("height", |b| b.height_cm),
    ("bust_circ", |b| b.bust_cm),
    ("waist_circ", |b| b.waist_cm),
    ("hip_circ", |b| b.hips_cm),
];

/// A measurement's value: millimetres for a length, else as written (an
/// angle, a ratio).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    pub value: f64,
    pub length: bool,
}

/// Measurement values by name (`bust_circ`, `@custom`).
pub type Values = BTreeMap<String, Value>;

/// One size of a multisize file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileSize {
    /// Its label, or its value on the size dimension.
    pub name: String,
    /// The file's base size.
    pub base: bool,
    pub values: Values,
}

/// A measurement file.
#[derive(Clone, Debug, PartialEq)]
pub enum Measurements {
    Individual(Individual),
    Multisize(Multisize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Individual {
    mm_per_unit: f64,
    /// 0 for a woman, 1 for a man, as the file says.
    masculinity: Option<f64>,
    /// Name, formula (in the file's unit), whether it is a length.
    entries: Vec<(String, String, bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Multisize {
    mm_per_unit: f64,
    dims: Vec<Dimension>,
    /// Y, W and Z values are half circumferences.
    full_circumference: bool,
    entries: Vec<Shifted>,
}

#[derive(Clone, Debug, PartialEq)]
struct Dimension {
    /// `x` height, `y` chest or size, `w` waist, `z` hips.
    kind: char,
    base: f64,
    step: f64,
    /// Steps below and above the base the file offers.
    range: (i64, i64),
    labels: Vec<(f64, String)>,
}

#[derive(Clone, Debug, PartialEq)]
struct Shifted {
    name: String,
    base: f64,
    shifts: Vec<f64>,
    /// Corrections by the dimensions' values joined with `;`.
    corrections: Vec<(String, f64)>,
    length: bool,
}

fn attr_f64(node: roxmltree::Node, name: &str) -> Option<f64> {
    node.attribute(name)?.trim().parse().ok()
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|c| c.has_tag_name(name))
}

fn malformed(detail: impl Into<String>) -> ValError {
    ValError::Malformed {
        detail: detail.into(),
    }
}

impl Measurements {
    /// Read a measurement file (individual or multisize, either dialect).
    pub fn parse(bytes: &[u8]) -> Result<Measurements, ValError> {
        let text = std::str::from_utf8(bytes).map_err(|_| malformed("not UTF-8 text"))?;
        let doc = roxmltree::Document::parse(text.trim_start_matches('\u{feff}'))
            .map_err(|e| malformed(e.to_string()))?;
        let r = root(&doc);
        let unit = child(r, "unit").and_then(|u| u.text()).unwrap_or("cm");
        let mm = mm_per_unit(unit)?;
        let list =
            child(r, "body-measurements").ok_or_else(|| malformed("no body-measurements"))?;
        let ms = list
            .children()
            .filter(|m| m.has_tag_name("m") && m.attribute("type") != Some("separator"));
        let length = |m: roxmltree::Node| {
            m.attribute("specialUnits") != Some("true")
                && !m.attribute("name").is_some_and(|n| n.contains("angle"))
        };
        match r.tag_name().name() {
            "vit" | "smis" => Ok(Measurements::Individual(Individual {
                mm_per_unit: mm,
                masculinity: child(r, "personal")
                    .and_then(|p| child(p, "gender"))
                    .and_then(|g| match g.text()?.trim() {
                        "male" => Some(1.0),
                        "female" => Some(0.0),
                        _ => None,
                    }),
                entries: ms
                    .filter_map(|m| {
                        Some((
                            m.attribute("name")?.to_owned(),
                            m.attribute("value").unwrap_or("0").to_owned(),
                            length(m),
                        ))
                    })
                    .collect(),
            })),
            "vst" | "smms" => {
                let (dims, full) = match child(r, "dimensions") {
                    Some(d) => (
                        d.children()
                            .filter(|c| c.has_tag_name("dimension"))
                            .map(dimension)
                            .collect::<Vec<_>>(),
                        d.attribute("fullCircumference") == Some("true"),
                    ),
                    None => (old_dimensions(r, mm), false),
                };
                let old = child(r, "dimensions").is_none();
                let entries = ms
                    .filter_map(|m| {
                        let shifts = if old {
                            vec![
                                attr_f64(m, "height_increase").unwrap_or(0.0),
                                attr_f64(m, "size_increase").unwrap_or(0.0),
                            ]
                        } else {
                            ["shiftA", "shiftB", "shiftC"]
                                .iter()
                                .take(dims.len())
                                .map(|a| attr_f64(m, a).unwrap_or(0.0))
                                .collect()
                        };
                        let corrections = child(m, "corrections")
                            .map(|c| {
                                c.children()
                                    .filter(|c| c.has_tag_name("correction"))
                                    .filter_map(|c| {
                                        Some((
                                            c.attribute("coordinates")?.to_owned(),
                                            attr_f64(c, "correction")?,
                                        ))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        Some(Shifted {
                            name: m.attribute("name")?.to_owned(),
                            base: attr_f64(m, "base").unwrap_or(0.0),
                            shifts,
                            corrections,
                            length: length(m),
                        })
                    })
                    .collect();
                if dims.is_empty() {
                    return Err(malformed("no dimensions"));
                }
                Ok(Measurements::Multisize(Multisize {
                    mm_per_unit: mm,
                    dims,
                    full_circumference: full,
                    entries,
                }))
            }
            other => Err(malformed(format!("not a measurement file (`{other}`)"))),
        }
    }

    /// 0 for a woman, 1 for a man, when an individual file says so.
    pub fn masculinity(&self) -> Option<f64> {
        match self {
            Measurements::Individual(m) => m.masculinity,
            Measurements::Multisize(_) => None,
        }
    }

    /// The values for `body` (none: the file's own, at its base size).
    pub fn values(&self, body: Option<&Body>) -> Result<Values, ValError> {
        match self {
            Measurements::Individual(m) => m.values(body),
            Measurements::Multisize(m) => {
                let k = match body {
                    Some(b) => m.fit(b),
                    None => vec![0.0; m.dims.len()],
                };
                Ok(m.at(&k))
            }
        }
    }

    /// The sizes a multisize file offers along its size dimension (chest,
    /// else the first), the others at their base; none for an individual
    /// file.
    pub fn sizes(&self) -> Vec<FileSize> {
        let Measurements::Multisize(m) = self else {
            return Vec::new();
        };
        let Some(d) = m
            .dims
            .iter()
            .position(|d| d.kind == 'y')
            .or(if m.dims.is_empty() { None } else { Some(0) })
        else {
            return Vec::new();
        };
        let Some(dim) = m.dims.get(d) else {
            return Vec::new();
        };
        (dim.range.0.min(0)..=dim.range.1.max(0))
            .map(|s| {
                let mut k = vec![0.0; m.dims.len()];
                if let Some(slot) = k.get_mut(d) {
                    *slot = s as f64;
                }
                let v = dim.base + s as f64 * dim.step;
                let name = dim
                    .labels
                    .iter()
                    .find(|(lv, _)| (lv - v).abs() < 1e-9)
                    .map(|(_, l)| l.clone())
                    .unwrap_or_else(|| {
                        let shown = if m.full_circumference && dim.kind != 'x' {
                            2.0 * v
                        } else {
                            v
                        };
                        number(shown)
                    });
                FileSize {
                    name,
                    base: s == 0,
                    values: m.at(&k),
                }
            })
            .collect()
    }
}

/// A number as the files print it: no trailing zeros.
fn number(v: f64) -> String {
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn dimension(d: roxmltree::Node) -> Dimension {
    let base = attr_f64(d, "base").unwrap_or(0.0);
    let step = attr_f64(d, "step").unwrap_or(0.0);
    let steps = |v: Option<f64>| match v {
        Some(v) if step > 0.0 => ((v - base) / step).round() as i64,
        _ => 0,
    };
    Dimension {
        kind: d
            .attribute("type")
            .and_then(|t| t.chars().next())
            .unwrap_or('y'),
        base,
        step,
        range: (steps(attr_f64(d, "min")), steps(attr_f64(d, "max"))),
        labels: child(d, "labels")
            .map(|l| {
                l.children()
                    .filter(|c| c.has_tag_name("label"))
                    .filter_map(|c| Some((attr_f64(c, "value")?, c.attribute("label")?.to_owned())))
                    .filter(|(_, l)| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// The old multisize layout: a base height (steps of 6 cm) and a base
/// size (half the chest, steps of 2 cm); the file does not say which
/// sizes it covers: two sizes each side of the base.
fn old_dimensions(r: roxmltree::Node, mm: f64) -> Vec<Dimension> {
    let base = |name: &str| child(r, name).and_then(|n| attr_f64(n, "base"));
    vec![
        Dimension {
            kind: 'x',
            base: base("height").unwrap_or(1760.0 / mm),
            step: 60.0 / mm,
            range: (0, 0),
            labels: Vec::new(),
        },
        Dimension {
            kind: 'y',
            base: base("size").unwrap_or(500.0 / mm),
            step: 20.0 / mm,
            range: (-2, 2),
            labels: Vec::new(),
        },
    ]
}

impl Individual {
    fn values(&self, body: Option<&Body>) -> Result<Values, ValError> {
        // In the file's unit while computing: formulas name each other.
        let mut raw: BTreeMap<String, f64> = BTreeMap::new();
        if let Some(b) = body {
            for (name, get) in BODY_NAMES {
                if self.entries.iter().any(|(n, _, _)| n == name) {
                    raw.insert(name.to_owned(), get(b) * 10.0 / self.mm_per_unit);
                }
            }
        }
        for (name, f, _) in &self.entries {
            if raw.contains_key(name) {
                continue;
            }
            let v = formula::eval(f, &|n| raw.get(n).copied())?;
            raw.insert(name.clone(), v);
        }
        Ok(self
            .entries
            .iter()
            .filter_map(|(name, _, length)| {
                let v = *raw.get(name)?;
                Some((
                    name.clone(),
                    Value {
                        value: if *length { v * self.mm_per_unit } else { v },
                        length: *length,
                    },
                ))
            })
            .collect())
    }
}

impl Multisize {
    /// Values at `k` steps from the base along each dimension.
    fn at(&self, k: &[f64]) -> Values {
        let coords = self
            .dims
            .iter()
            .zip(k)
            .map(|(d, k)| number(d.base + k * d.step))
            .collect::<Vec<_>>()
            .join(";");
        self.entries
            .iter()
            .map(|e| {
                let mut v = e.base + e.shifts.iter().zip(k).map(|(s, k)| s * k).sum::<f64>();
                if let Some((_, c)) = e.corrections.iter().find(|(at, _)| *at == coords) {
                    v += c;
                }
                (
                    e.name.clone(),
                    Value {
                        value: if e.length { v * self.mm_per_unit } else { v },
                        length: e.length,
                    },
                )
            })
            .collect()
    }

    /// The steps along each dimension whose values come closest to the
    /// body's height, bust, waist and hips (least squares).
    fn fit(&self, body: &Body) -> Vec<f64> {
        let n = self.dims.len();
        let mut rows: Vec<(Vec<f64>, f64)> = Vec::new();
        for (name, get) in BODY_NAMES {
            if let Some(e) = self.entries.iter().find(|e| e.name == name && e.length) {
                let target = get(body) * 10.0 / self.mm_per_unit - e.base;
                let mut a = e.shifts.clone();
                a.resize(n, 0.0);
                rows.push((a, target));
            }
        }
        // Normal equations, with a small damping so that a dimension no
        // measurement moves stays at its base.
        let mut m = vec![vec![0.0; n + 1]; n];
        for (a, t) in &rows {
            for i in 0..n {
                let ai = a.get(i).copied().unwrap_or(0.0);
                if let Some(row) = m.get_mut(i) {
                    for j in 0..n {
                        if let Some(c) = row.get_mut(j) {
                            *c += ai * a.get(j).copied().unwrap_or(0.0);
                        }
                    }
                    if let Some(c) = row.get_mut(n) {
                        *c += ai * t;
                    }
                }
            }
        }
        for (i, row) in m.iter_mut().enumerate() {
            if let Some(c) = row.get_mut(i) {
                *c += 1e-9;
            }
        }
        solve(m).unwrap_or_else(|| vec![0.0; n])
    }
}

/// Solve an augmented `n × (n+1)` system by Gaussian elimination.
fn solve(mut m: Vec<Vec<f64>>) -> Option<Vec<f64>> {
    let n = m.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|a, b| {
            let va = m.get(*a).and_then(|r| r.get(col)).map_or(0.0, |v| v.abs());
            let vb = m.get(*b).and_then(|r| r.get(col)).map_or(0.0, |v| v.abs());
            va.total_cmp(&vb)
        })?;
        m.swap(col, pivot);
        let row = m.get(col)?.clone();
        let p = *row.get(col)?;
        if p.abs() < 1e-12 {
            return None;
        }
        for (r, other) in m.iter_mut().enumerate() {
            if r == col {
                continue;
            }
            let f = other.get(col).copied().unwrap_or(0.0) / p;
            for (c, v) in other.iter_mut().enumerate() {
                *v -= f * row.get(c).copied().unwrap_or(0.0);
            }
        }
    }
    m.iter()
        .enumerate()
        .map(|(i, r)| Some(r.get(n)? / r.get(i)?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<vit>
  <version>0.6.2</version>
  <unit>cm</unit>
  <body-measurements>
    <m name="height" value="168"/>
    <m name="bust_circ" value="88"/>
    <m name="waist_circ" value="70"/>
    <m name="hip_circ" value="96"/>
    <m name="bust_arc_f" value="bust_circ/2 + 2"/>
    <m name="@dart_angle" value="12" specialUnits="true"/>
  </body-measurements>
</vit>"#;

    const VST: &str = r#"<vst>
  <version>0.4.4</version>
  <unit>cm</unit>
  <size base="48"/>
  <height base="176"/>
  <body-measurements>
    <m name="height" base="176" height_increase="6" size_increase="0"/>
    <m name="bust_circ" base="96" height_increase="0" size_increase="4"/>
    <m name="waist_circ" base="84" height_increase="0" size_increase="4"/>
    <m name="neck_circ" base="38" height_increase="0" size_increase="1"/>
  </body-measurements>
</vst>"#;

    fn body(height: f64, bust: f64, waist: f64, hips: f64) -> Body {
        Body {
            masculinity: 0.0,
            height_cm: height,
            bust_cm: bust,
            waist_cm: waist,
            hips_cm: hips,
            age_years: None,
        }
    }

    #[test]
    fn individual_values_and_body() {
        let m = Measurements::parse(VIT.as_bytes()).unwrap();
        let v = m.values(None).unwrap();
        assert_eq!(v["bust_arc_f"].value, 460.0);
        assert_eq!(v["@dart_angle"].value, 12.0);
        assert!(!v["@dart_angle"].length);
        let v = m.values(Some(&body(170.0, 100.0, 80.0, 104.0))).unwrap();
        assert_eq!(v["bust_circ"].value, 1000.0);
        assert_eq!(v["bust_arc_f"].value, 520.0);
        assert!(m.sizes().is_empty());
    }

    #[test]
    fn multisize_fits_the_body() {
        let m = Measurements::parse(VST.as_bytes()).unwrap();
        let v = m.values(None).unwrap();
        assert_eq!(v["neck_circ"].value, 380.0);
        // Two sizes up: chest +8 cm, neck +2 cm.
        let v = m.values(Some(&body(176.0, 104.0, 92.0, 100.0))).unwrap();
        assert!((v["neck_circ"].value - 400.0).abs() < 1e-6);
        assert!((v["height"].value - 1760.0).abs() < 1e-6);
        let sizes = m.sizes();
        assert_eq!(
            sizes.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["44", "46", "48", "50", "52"]
        );
        assert!((sizes[0].values["bust_circ"].value - 880.0).abs() < 1e-9);
    }
}
