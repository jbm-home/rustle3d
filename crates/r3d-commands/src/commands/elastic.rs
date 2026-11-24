//! Elastic and shirring (`docs/specs/31-construction-details-2.md`).

use r3d_core::detail::{ELASTIC_RATIO, ELASTIC_WIDTH_MM};
use r3d_core::seam::Span;
use r3d_core::{
    Detail, DetailId, EdgeId, Elastic, InternalLine, PanelId, PathSegment, Project, SeamSide,
    Topstitch, Transaction,
};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::get_panel;
use super::seam::SeamSideSpec;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// An elastic's ratio when none is given: three quarters of the side.
pub const DEFAULT_RATIO: f64 = 0.75;
/// An elastic's width when none is given (mm).
pub const DEFAULT_WIDTH_MM: f64 = 10.0;
/// Shirring: the gap between rows (mm) and the first row's distance from
/// its edge when none is given, and the shirring elastic's width.
pub const SHIRRING_GAP_MM: f64 = 15.0;
pub const SHIRRING_WIDTH_MM: f64 = 2.0;
/// Shirring rows stop this far inside the outline (mm): a line on the
/// outline could not be meshed.
const SHIRRING_MARGIN_MM: f64 = 1.0;
/// The most shirring rows in one go.
pub const MAX_SHIRRING_ROWS: u32 = 30;

fn ratio(field: &'static str, r: f64) -> Result<f64, CommandError> {
    finite(field, &[r])?;
    if (ELASTIC_RATIO.0..=ELASTIC_RATIO.1).contains(&r) {
        Ok(r)
    } else {
        Err(CommandError::invalid(
            field,
            Invalid::Range(ELASTIC_RATIO.0, ELASTIC_RATIO.1),
        ))
    }
}

fn width(mm: f64) -> Result<f64, CommandError> {
    finite("width_mm", &[mm])?;
    if (ELASTIC_WIDTH_MM.0..=ELASTIC_WIDTH_MM.1).contains(&mm) {
        Ok(mm)
    } else {
        Err(CommandError::invalid(
            "width_mm",
            Invalid::Range(ELASTIC_WIDTH_MM.0, ELASTIC_WIDTH_MM.1),
        ))
    }
}

/// The ratio from `ratio` or `relaxed_mm` (one at most) for a side
/// `length` long; `None` when neither is given.
fn ratio_of(
    ratio_in: Option<f64>,
    relaxed_mm: Option<f64>,
    length: f64,
) -> Result<Option<f64>, CommandError> {
    match (ratio_in, relaxed_mm) {
        (Some(_), Some(_)) => Err(CommandError::invalid("relaxed_mm", Invalid::Format)),
        (Some(r), None) => ratio("ratio", r).map(Some),
        (None, Some(mm)) => {
            finite("relaxed_mm", &[mm])?;
            if length <= 0.0 {
                return Err(CommandError::invalid("side", Invalid::Empty));
            }
            ratio("relaxed_mm", mm / length).map(Some)
        }
        (None, None) => Ok(None),
    }
}

pub(crate) fn get_elastic(p: &Project, id: DetailId) -> Result<Elastic, CommandError> {
    match p.details().get(&id) {
        Some(Detail::Elastic(e)) => Ok(e.clone()),
        Some(_) => Err(CommandError::invalid("elastic", Invalid::OtherKind)),
        None => Err(CommandError::not_found(id)),
    }
}

/// Two lists of stretches share some length.
fn overlap(a: &[Span], b: &[Span]) -> bool {
    a.iter().any(|x| {
        b.iter()
            .any(|y| x.track == y.track && x.from.max(y.from) < x.to.min(y.to) - 1e-6)
    })
}

/// No other elastic on any stretch of `side`.
fn free_of_elastics(
    p: &Project,
    side: &SeamSide,
    except: Option<DetailId>,
) -> Result<(), CommandError> {
    let spans = p.side_spans(side).unwrap_or_default();
    let taken = p.elastics().any(|(id, e)| {
        Some(id) != except && overlap(&spans, &p.side_spans(&e.side).unwrap_or_default())
    });
    if taken {
        Err(CommandError::invalid("side", Invalid::HasElastic))
    } else {
        Ok(())
    }
}

/// Add an elastic along a side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElasticAdd {
    /// Where it is sewn, as a seam's side.
    pub side: SeamSideSpec,
    /// Its relaxed length over the side's (0.2–1). Omitted (and no
    /// `relaxed_mm`): 0.75.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    /// Or its relaxed length (mm), turned into the ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relaxed_mm: Option<f64>,
    /// Its width (2–50 mm): how strongly it pulls. Omitted: 10 mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
}

impl Described for ElasticAdd {
    const SUMMARY: &'static str = "Sew an elastic along a side (given as a seam's: a whole `edge`, consecutive outline `edges` or an internal `line`, maybe only `from_mm`/`to_mm` along it): the drape gathers it to `ratio` × its length (0.2–1, default 0.75), or to `relaxed_mm`. `width_mm` (2–50, default 10) sets how strongly it pulls. The ratio is kept when the side changes length or in other sizes. A stretch may carry one elastic; it may also be sewn. Returns the elastic (a detail).";
}

impl CommandSpec for ElasticAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let side = self.side.side("side", p)?;
        let length = p.side_length(&side).unwrap_or_default();
        let ratio = ratio_of(self.ratio, self.relaxed_mm, length)?.unwrap_or(DEFAULT_RATIO);
        let width_mm = width(self.width_mm.unwrap_or(DEFAULT_WIDTH_MM))?;
        free_of_elastics(p, &side, None)?;
        let id = tx.new_detail_id();
        tx.put_detail(
            id,
            Detail::Elastic(Elastic {
                side,
                ratio,
                width_mm,
            }),
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Change an elastic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElasticSet {
    pub elastic: DetailId,
    /// Its relaxed length over the side's (0.2–1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    /// Or its relaxed length (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relaxed_mm: Option<f64>,
    /// Its width (2–50 mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
}

impl Described for ElasticSet {
    const SUMMARY: &'static str = "Change an elastic: its `ratio` (or `relaxed_mm`) and its `width_mm`; what is omitted stays. A running drape follows without starting again.";
}

impl CommandSpec for ElasticSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let e = get_elastic(p, self.elastic)?;
        let length = p.side_length(&e.side).unwrap_or_default();
        let ratio = ratio_of(self.ratio, self.relaxed_mm, length)?.unwrap_or(e.ratio);
        let width_mm = match self.width_mm {
            Some(w) => width(w)?,
            None => e.width_mm,
        };
        tx.put_detail(
            self.elastic,
            Detail::Elastic(Elastic {
                ratio,
                width_mm,
                ..e
            }),
        );
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("elastic.set:{}", self.elastic))
    }
}

/// Remove an elastic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElasticRemove {
    pub elastic: DetailId,
}

impl Described for ElasticRemove {
    const SUMMARY: &'static str = "Remove an elastic: its side no longer gathers.";
}

impl CommandSpec for ElasticRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_elastic(tx.project(), self.elastic)?;
        tx.remove_detail(self.elastic);
        Ok(CommandOutput::default())
    }
}

/// Shirr a piece: parallel rows of elastic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElasticShirr {
    pub panel: PanelId,
    /// The outline edge the rows run along (parallel to its chord).
    pub edge: EdgeId,
    /// How many rows (1–30).
    pub rows: u32,
    /// The first row's distance from the edge (mm). Omitted: 15 mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_mm: Option<f64>,
    /// The distance between rows (mm). Omitted: 15 mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_mm: Option<f64>,
    /// Each row's elastic ratio (0.2–1). Omitted: 0.75.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    /// The elastic's width (2–50 mm). Omitted: 2 mm (shirring elastic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
}

impl Described for ElasticShirr {
    const SUMMARY: &'static str = "Shirr a piece: `rows` parallel rows of elastic across it, along `edge`'s chord, the first `first_mm` inside it (default 15), then every `gap_mm` (default 15), each an internal line with its elastic (`ratio`, default 0.75; `width_mm`, default 2), stopping 1 mm short of the outline. One undo step. Returns the lines and the elastics.";
}

/// Where the straight line through `origin` along `dir` is inside the
/// loops (even–odd): the stretches, as distances along it.
fn inside_stretches(loops: &[Vec<DVec2>], origin: DVec2, dir: DVec2) -> Vec<(f64, f64)> {
    let mut hits: Vec<f64> = Vec::new();
    for pts in loops {
        let n = pts.len();
        for i in 0..n {
            let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else {
                continue;
            };
            let e = *b - *a;
            let denom = dir.perp_dot(e);
            if denom.abs() < 1e-12 {
                continue;
            }
            // origin + t·dir = a + s·e
            let w = *a - origin;
            let t = w.perp_dot(e) / denom;
            let s = w.perp_dot(dir) / denom;
            if (0.0..1.0).contains(&s) {
                hits.push(t);
            }
        }
    }
    hits.sort_by(f64::total_cmp);
    hits.as_chunks::<2>()
        .0
        .iter()
        .map(|[a, b]| (*a, *b))
        .collect()
}

impl CommandSpec for ElasticShirr {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        get_panel(p, self.panel)?;
        let edge = p
            .edge(self.edge)
            .ok_or_else(|| CommandError::not_found(self.edge))?;
        if edge.panel != self.panel {
            return Err(CommandError::invalid("edge", Invalid::OtherPanel));
        }
        if !(1..=MAX_SHIRRING_ROWS).contains(&self.rows) {
            return Err(CommandError::invalid(
                "rows",
                Invalid::Range(1.0, f64::from(MAX_SHIRRING_ROWS)),
            ));
        }
        let first = self.first_mm.unwrap_or(SHIRRING_GAP_MM);
        let gap = self.gap_mm.unwrap_or(SHIRRING_GAP_MM);
        finite("first_mm", &[first])?;
        finite("gap_mm", &[gap])?;
        if first <= 0.0 {
            return Err(CommandError::invalid("first_mm", Invalid::NotPositive));
        }
        if gap <= 0.0 {
            return Err(CommandError::invalid("gap_mm", Invalid::NotPositive));
        }
        let ratio = ratio("ratio", self.ratio.unwrap_or(DEFAULT_RATIO))?;
        let width_mm = width(self.width_mm.unwrap_or(SHIRRING_WIDTH_MM))?;
        let (Some(a), Some(b)) = (p.point(edge.start), p.point(edge.end)) else {
            return Err(CommandError::not_found(self.edge));
        };
        let dir = (b.pos - a.pos)
            .try_normalize()
            .ok_or_else(|| CommandError::invalid("edge", Invalid::ZeroVector))?;
        // Outlines turn counter-clockwise: the inside is on the left.
        let inward = dir.perp();
        let mut loops = vec![
            p.panel_outline(self.panel)
                .ok_or_else(|| CommandError::not_found(self.panel))?,
        ];
        loops.extend(p.hole_outlines(self.panel));
        let mut rows: Vec<(DVec2, DVec2)> = Vec::new();
        for k in 0..self.rows {
            let origin = a.pos + inward * (first + gap * f64::from(k));
            for (t0, t1) in inside_stretches(&loops, origin, dir) {
                let (t0, t1) = (t0 + SHIRRING_MARGIN_MM, t1 - SHIRRING_MARGIN_MM);
                if t1 - t0 > 2.0 * SHIRRING_GAP_MM.min(gap) {
                    rows.push((origin + dir * t0, origin + dir * t1));
                }
            }
        }
        if rows.is_empty() {
            return Err(CommandError::invalid("first_mm", Invalid::OutsidePiece));
        }
        let mut created = Vec::new();
        for (start, end) in rows {
            let line = tx.new_internal_line_id();
            tx.put_internal_line(
                line,
                InternalLine {
                    panel: self.panel,
                    start,
                    segments: vec![PathSegment::Line { to: end }],
                },
            );
            let elastic = tx.new_detail_id();
            tx.put_detail(
                elastic,
                Detail::Elastic(Elastic {
                    side: SeamSide {
                        edges: Vec::new(),
                        line: Some(line),
                        from_mm: None,
                        to_mm: None,
                        reversed: false,
                    },
                    ratio,
                    width_mm,
                }),
            );
            created.push(line.into());
            created.push(elastic.into());
        }
        Ok(CommandOutput { created })
    }
}

/// After a step: drop the details whose sides are gone. The changes join
/// the step's undo step.
pub(crate) fn settle_details(tx: &mut Transaction<'_>) {
    let p = tx.project();
    let gone: Vec<DetailId> = p
        .details()
        .iter()
        .filter(|(_, d)| match d {
            Detail::Elastic(Elastic { side, .. }) | Detail::Topstitch(Topstitch { side, .. }) => {
                !p.side_length(side).is_some_and(|len| len > 0.0)
            }
        })
        .map(|(id, _)| *id)
        .collect();
    for id in gone {
        tx.remove_detail(id);
    }
}

/// Each elastic's relaxed length (mm) in each size of the range (its
/// name), or in the drawn size alone (no name) without one.
pub fn elastic_lengths(
    p: &Project,
) -> std::collections::BTreeMap<DetailId, Vec<(Option<String>, f64)>> {
    let sized: Vec<(Option<String>, std::borrow::Cow<'_, Project>)> = match p.grading() {
        Some(g) => g
            .sizes
            .iter()
            .enumerate()
            .map(|(i, s)| (Some(s.name.clone()), std::borrow::Cow::Owned(p.at_size(i))))
            .collect(),
        None => vec![(None, std::borrow::Cow::Borrowed(p))],
    };
    let mut out: std::collections::BTreeMap<DetailId, Vec<(Option<String>, f64)>> =
        std::collections::BTreeMap::new();
    for (name, at) in &sized {
        for (id, e) in at.elastics() {
            if let Some(len) = at.elastic_length(e) {
                out.entry(id).or_default().push((name.clone(), len));
            }
        }
    }
    out
}
