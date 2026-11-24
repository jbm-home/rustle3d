//! Construction details that change the cloth where they are
//! (`docs/specs/31-construction-details-2.md`): elastic, topstitching,
//! interfacing, quilting.

use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::{EdgeId, InternalLineId, PanelId};
use crate::model::{Project, SeamSide};

/// The smallest and largest elastic ratios (relaxed length over the
/// side's length).
pub const ELASTIC_RATIO: (f64, f64) = (0.2, 1.0);
/// The narrowest and widest elastics (mm).
pub const ELASTIC_WIDTH_MM: (f64, f64) = (2.0, 50.0);
/// How hard an elastic pulls: newtons per unit strain per millimetre of
/// width (≈ 0.8 N per mm when stretched to twice its length; general
/// knowledge of knitted and braided elastics, an estimate to calibrate,
/// docs/physics.md "Elastic").
pub const ELASTIC_N_PER_MM: f64 = 0.8;

/// Topstitching: at most this many rows.
pub const TOPSTITCH_ROWS: u8 = 3;
/// How far its first row may be from its side (mm): from the side itself
/// (a line) or 1 mm (an outline edge, inside the piece) to this.
pub const TOPSTITCH_OFFSET_MM: f64 = 50.0;
/// The smallest gap between rows (mm), and the largest.
pub const TOPSTITCH_GAP_MM: (f64, f64) = (1.0, 30.0);
/// The shortest and longest stitches (mm).
pub const STITCH_MM: (f64, f64) = (1.0, 8.0);

/// A detail on a piece: one entity kind for what spec 31 adds, each its
/// own variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Detail {
    /// An elastic along a side, gathering it.
    Elastic(Elastic),
    /// Rows of stitches along a side, seen on the right side.
    Topstitch(Topstitch),
    /// A fused interfacing on part of a piece.
    Interfacing(Interfacing),
    /// Quilting over a padding on part of a piece.
    Quilting(Quilting),
}

/// Quilting: the closest and widest spacing of its lines (mm).
pub const QUILT_SPACING_MM: (f64, f64) = (15.0, 150.0);
/// The thinnest and thickest padding (mm).
pub const LOFT_MM: (f64, f64) = (1.0, 30.0);
/// The lightest and heaviest padding (g/m²).
pub const PADDING_GSM: (f64, f64) = (10.0, 600.0);

/// How a quilting's lines run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuiltPattern {
    /// Parallel lines.
    Channels,
    /// Two families of lines, at the angle each side of the grainline.
    #[default]
    Diamonds,
}

/// A quilting: lines stitched through the cloth, a padding and a lining
/// (one layer in the simulation, docs/physics.md "Composite cloth"), over
/// the whole piece or inside a closed internal line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Quilting {
    pub panel: PanelId,
    /// Inside this closed line of the piece; `None`: the whole piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<InternalLineId>,
    pub pattern: QuiltPattern,
    /// The lines' angle from the grainline (degrees); for diamonds, each
    /// family's, one each side.
    pub angle_deg: f64,
    /// The distance between parallel lines (mm).
    pub spacing_mm: f64,
    /// The padding's thickness (mm).
    pub loft_mm: f64,
    /// The padding's weight (g/m²).
    pub padding_gsm: f64,
    /// The stitches' length (mm).
    pub stitch_mm: f64,
    /// The thread's colour (`#RRGGBB`); `None`: a darker shade of the
    /// garment's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Quilting {
    /// Its values within their ranges.
    pub fn valid(&self) -> bool {
        (QUILT_SPACING_MM.0..=QUILT_SPACING_MM.1).contains(&self.spacing_mm)
            && (LOFT_MM.0..=LOFT_MM.1).contains(&self.loft_mm)
            && (PADDING_GSM.0..=PADDING_GSM.1).contains(&self.padding_gsm)
            && (STITCH_MM.0..=STITCH_MM.1).contains(&self.stitch_mm)
            && self.angle_deg.is_finite()
            && self.color.as_deref().is_none_or(|c| parse_hex(c).is_some())
    }

    /// The stitched lines' directions on the pattern, for a piece whose
    /// grainline turns `grain_deg`.
    fn directions(&self, grain_deg: f64) -> Vec<DVec2> {
        let at = |a: f64| DVec2::from_angle((grain_deg + a).to_radians());
        match self.pattern {
            QuiltPattern::Channels => vec![at(self.angle_deg)],
            QuiltPattern::Diamonds => vec![at(self.angle_deg), at(-self.angle_deg)],
        }
    }
}

/// The widest band of interfacing along edges (mm).
pub const INTERFACING_BAND_MM: f64 = 300.0;

/// How an interfacing is made: how it stretches.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InterfacingFabric {
    #[default]
    Woven,
    NonWoven,
    /// Stretches across, with the cloth.
    Knit,
}

/// How heavy an interfacing is.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InterfacingWeight {
    Light,
    #[default]
    Medium,
    Heavy,
}

/// Where an interfacing is fused.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InterfacingArea {
    /// The whole piece.
    Whole,
    /// A band `width_mm` wide inside consecutive outline edges, in their
    /// order around the piece (a facing, a waistband, a front band).
    Band { edges: Vec<EdgeId>, width_mm: f64 },
    /// Inside a closed internal line (its ends together).
    Inside { line: InternalLineId },
}

/// A fused interfacing on part of a piece: it makes the cloth there
/// heavier, thicker and much stiffer to bend (docs/physics.md,
/// "Composite cloth").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Interfacing {
    pub panel: PanelId,
    pub area: InterfacingArea,
    #[serde(default)]
    pub fabric: InterfacingFabric,
    #[serde(default)]
    pub weight: InterfacingWeight,
}

/// The thread a topstitch is sewn with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Thread {
    /// The garment's sewing thread.
    #[default]
    Normal,
    /// Topstitching thread, about twice as thick.
    Heavy,
}

impl Thread {
    /// How wide its stitches look (mm).
    pub fn width_mm(self) -> f64 {
        match self {
            Thread::Normal => 0.4,
            Thread::Heavy => 0.8,
        }
    }
}

/// Rows of stitches along a side (as an elastic's): the first
/// `offset_mm` from it, the next ones `gap_mm` further. On outline edges
/// the rows are inside the piece; on an internal line, on its left
/// (walking from its start) when the offset is positive, its right when
/// negative.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Topstitch {
    pub side: SeamSide,
    /// 1 to 3.
    pub rows: u8,
    pub offset_mm: f64,
    pub gap_mm: f64,
    /// The length of a stitch (1–8 mm).
    pub stitch_mm: f64,
    /// The thread's colour (`#RRGGBB`); `None`: a darker shade of the
    /// garment's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub thread: Thread,
}

impl Topstitch {
    /// Its values within their ranges (an outline side's rows inside the
    /// piece).
    pub fn valid(&self) -> bool {
        let on_line = self.side.line.is_some();
        let least = if on_line { -TOPSTITCH_OFFSET_MM } else { 1.0 };
        (1..=TOPSTITCH_ROWS).contains(&self.rows)
            && (least..=TOPSTITCH_OFFSET_MM).contains(&self.offset_mm)
            && (TOPSTITCH_GAP_MM.0..=TOPSTITCH_GAP_MM.1).contains(&self.gap_mm)
            && (STITCH_MM.0..=STITCH_MM.1).contains(&self.stitch_mm)
            && self.color.as_deref().is_none_or(|c| parse_hex(c).is_some())
    }

    /// The distance of the last row from its side (mm, signed as the
    /// offset).
    pub fn reach_mm(&self) -> f64 {
        let sign = if self.offset_mm < 0.0 { -1.0 } else { 1.0 };
        self.offset_mm + sign * self.gap_mm * f64::from(self.rows.saturating_sub(1))
    }
}

/// `#RRGGBB` as `0xRRGGBB`.
pub fn parse_hex(c: &str) -> Option<u32> {
    c.strip_prefix('#')
        .filter(|d| d.len() == 6)
        .and_then(|d| u32::from_str_radix(d, 16).ok())
}

/// `rgb` darkened: a thread a shade darker than its cloth.
fn darker(rgb: u32) -> u32 {
    let ch = |s: u32| (((rgb >> s) & 0xFF) as f64 * 0.6).round() as u32;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// An interfacing's bounds stay this far from the piece's outline (mm).
const BOUND_CLEAR_MM: f64 = 1.0;

/// `lines` with a point added on each where another crosses it: the
/// same point on both (computed once), so a mesh joins them there.
fn with_crossings(lines: &[Vec<DVec2>]) -> Vec<Vec<DVec2>> {
    // For each line and segment, the crossings on it: where along it, and
    // the point.
    let mut cuts: std::collections::HashMap<(usize, usize), Vec<(f64, DVec2)>> =
        std::collections::HashMap::new();
    for (k, line) in lines.iter().enumerate() {
        for (j, other) in lines.iter().enumerate().skip(k + 1) {
            for (i, w) in line.windows(2).enumerate() {
                let &[a, b] = w else { continue };
                let d = b - a;
                for (m, v) in other.windows(2).enumerate() {
                    let &[c, e] = v else { continue };
                    let s = e - c;
                    let denom = d.perp_dot(s);
                    if denom.abs() < 1e-12 {
                        continue;
                    }
                    let w = c - a;
                    let t = w.perp_dot(s) / denom;
                    let u = w.perp_dot(d) / denom;
                    if t > 1e-9 && t < 1.0 - 1e-9 && u > 1e-9 && u < 1.0 - 1e-9 {
                        let p = a + d * t;
                        cuts.entry((k, i)).or_default().push((t, p));
                        cuts.entry((j, m)).or_default().push((u, p));
                    }
                }
            }
        }
    }
    lines
        .iter()
        .enumerate()
        .map(|(k, line)| {
            let mut out: Vec<DVec2> = line.first().copied().into_iter().collect();
            for (i, w) in line.windows(2).enumerate() {
                let &[_, b] = w else { continue };
                if let Some(list) = cuts.get_mut(&(k, i)) {
                    list.sort_by(|x, y| x.0.total_cmp(&y.0));
                    out.extend(list.iter().map(|(_, p)| *p));
                }
                out.push(b);
            }
            out
        })
        .collect()
}

/// `line` with points added so none are more than `step` apart.
fn densified(line: &[DVec2], step: f64) -> Vec<DVec2> {
    let mut out = Vec::new();
    for w in line.windows(2) {
        if let [a, b] = w {
            let n = ((a.distance(*b) / step).ceil() as usize).max(1);
            out.extend((0..n).map(|k| a.lerp(*b, k as f64 / n as f64)));
        }
    }
    out.extend(line.last());
    out
}

/// The parts of `run` farther than `clear` from the loop `outline`.
fn split_near(run: &[DVec2], outline: &[DVec2], clear: f64) -> Vec<Vec<DVec2>> {
    let far = |p: DVec2| r3d_geom::polygon::distance_to_loop(outline, p) > clear;
    let mut out = Vec::new();
    let mut part: Vec<DVec2> = Vec::new();
    for p in run {
        if far(*p) {
            part.push(*p);
        } else if part.len() > 1 {
            out.push(std::mem::take(&mut part));
        } else {
            part.clear();
        }
    }
    if part.len() > 1 {
        out.push(part);
    }
    out
}

/// Rows stop this far inside the outline (mm).
const ROW_MARGIN_MM: f64 = 0.5;

/// The parts of `line` inside the first loop and outside the others
/// (even–odd), each shortened by [`ROW_MARGIN_MM`] at its ends.
fn clip_inside(line: &[DVec2], loops: &[Vec<DVec2>]) -> Vec<Vec<DVec2>> {
    let inside = |q: DVec2| {
        loops
            .iter()
            .filter(|l| r3d_geom::polygon::contains(l, q))
            .count()
            % 2
            == 1
    };
    let mut runs: Vec<Vec<DVec2>> = Vec::new();
    let mut run: Vec<DVec2> = Vec::new();
    for w in line.windows(2) {
        let &[a, b] = w else { continue };
        let d = b - a;
        // Where the segment crosses a loop, in order along it.
        let mut cuts = vec![0.0, 1.0];
        for l in loops {
            let n = l.len();
            for i in 0..n {
                let (Some(p), Some(q)) = (l.get(i), l.get((i + 1) % n)) else {
                    continue;
                };
                let e = *q - *p;
                let denom = d.perp_dot(e);
                if denom.abs() < 1e-12 {
                    continue;
                }
                let w = *p - a;
                let t = w.perp_dot(e) / denom;
                let u = w.perp_dot(d) / denom;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    cuts.push(t);
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        for c in cuts.windows(2) {
            let &[t0, t1] = c else { continue };
            if t1 - t0 < 1e-9 {
                continue;
            }
            let (p0, p1) = (a + d * t0, a + d * t1);
            if inside((p0 + p1) / 2.0) {
                if run.last().is_none_or(|q| q.distance(p0) > 1e-9) {
                    if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                    run.push(p0);
                }
                run.push(p1);
            } else if !run.is_empty() {
                runs.push(std::mem::take(&mut run));
            }
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs.into_iter()
        .filter_map(|r| {
            let len = crate::seam::polyline_length(&r);
            (len > 2.0 * ROW_MARGIN_MM)
                .then(|| crate::seam::sub_polyline(&r, ROW_MARGIN_MM, len - ROW_MARGIN_MM))
        })
        .filter(|r| r.len() > 1)
        .collect()
}

/// `line` moved `by` mm to its left (negative: its right), corners
/// mitred (at most twice `by` out).
fn offset_polyline(line: &[DVec2], by: f64) -> Vec<DVec2> {
    let n = line.len();
    (0..n)
        .filter_map(|i| {
            let p = *line.get(i)?;
            let before = i
                .checked_sub(1)
                .and_then(|j| line.get(j))
                .and_then(|q| (p - *q).try_normalize());
            let after = line.get(i + 1).and_then(|q| (*q - p).try_normalize());
            let (a, b) = match (before, after) {
                (Some(a), Some(b)) => (a, b),
                (Some(a), None) => (a, a),
                (None, Some(b)) => (b, b),
                (None, None) => return None,
            };
            let normal = (a.perp() + b.perp()).try_normalize().unwrap_or(a.perp());
            let cos = normal.dot(a.perp()).max(0.5);
            Some(p + normal * (by / cos))
        })
        .collect()
}

/// An elastic sewn along a side (an edge, consecutive outline edges or an
/// internal line, maybe a stretch of it), stretched to its length: the
/// side gathers to `ratio` times its length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Elastic {
    pub side: SeamSide,
    /// Relaxed length over the side's length (0.2–1).
    pub ratio: f64,
    /// Its width (mm): how strongly it pulls.
    pub width_mm: f64,
}

impl Elastic {
    /// Its stiffness: newtons per unit strain.
    pub fn stiffness_n(&self) -> f64 {
        ELASTIC_N_PER_MM * self.width_mm
    }

    /// Ratio and width within their ranges.
    pub fn valid(&self) -> bool {
        (ELASTIC_RATIO.0..=ELASTIC_RATIO.1).contains(&self.ratio)
            && (ELASTIC_WIDTH_MM.0..=ELASTIC_WIDTH_MM.1).contains(&self.width_mm)
    }
}

impl Detail {
    /// The piece it is on.
    pub fn panel(&self, p: &Project) -> Option<PanelId> {
        match self {
            Detail::Elastic(e) => p.side_panel(&e.side),
            Detail::Topstitch(t) => p.side_panel(&t.side),
            Detail::Interfacing(i) => Some(i.panel),
            Detail::Quilting(q) => Some(q.panel),
        }
    }
}

impl Project {
    /// Every elastic.
    pub fn elastics(&self) -> impl Iterator<Item = (crate::id::DetailId, &Elastic)> {
        self.details.iter().filter_map(|(id, d)| match d {
            Detail::Elastic(e) => Some((*id, e)),
            _ => None,
        })
    }

    /// Every topstitch.
    pub fn topstitches(&self) -> impl Iterator<Item = (crate::id::DetailId, &Topstitch)> {
        self.details.iter().filter_map(|(id, d)| match d {
            Detail::Topstitch(t) => Some((*id, t)),
            _ => None,
        })
    }

    /// A closed internal line of `panel` as a loop (mm, its end left
    /// out); `None` when it is open or on another piece.
    pub fn closed_line_outline(&self, panel: PanelId, line: InternalLineId) -> Option<Vec<DVec2>> {
        let l = self.internal_line(line)?;
        let mut pts = self.internal_line_polyline(line)?;
        let closed = pts.len() > 3
            && pts
                .first()
                .zip(pts.last())
                .is_some_and(|(a, b)| a.distance(*b) < 1e-6);
        if l.panel != panel || !closed {
            return None;
        }
        pts.pop();
        Some(pts)
    }

    /// Every quilting.
    pub fn quiltings(&self) -> impl Iterator<Item = (crate::id::DetailId, &Quilting)> {
        self.details.iter().filter_map(|(id, d)| match d {
            Detail::Quilting(q) => Some((*id, q)),
            _ => None,
        })
    }

    /// Where a quilting is (mm, a closed loop); `None` when its piece or
    /// line is gone or the line is open.
    pub fn quilting_outline(&self, q: &Quilting) -> Option<Vec<DVec2>> {
        match q.line {
            Some(line) => self.closed_line_outline(q.panel, line),
            None => self.panel_outline(q.panel),
        }
    }

    /// A quilting's thread colour (`0xRRGGBB`).
    pub fn quilting_rgb(&self, q: &Quilting) -> u32 {
        q.color.as_deref().and_then(parse_hex).unwrap_or_else(|| {
            let garment = self
                .panel(q.panel)
                .and_then(|p| self.garments().get(&p.garment));
            darker(garment.map_or(0x808080, |g| g.rgb()))
        })
    }

    /// A quilting's stitched lines on the pattern (mm), inside its area
    /// and its piece, spaced `spacing_mm` apart through the area's middle.
    /// Where lines of the two families cross, both have a point.
    pub fn quilting_lines(&self, q: &Quilting) -> Vec<Vec<DVec2>> {
        let (Some(panel), Some(area)) = (self.panel(q.panel), self.quilting_outline(q)) else {
            return Vec::new();
        };
        let Some(bb) = r3d_geom::polygon::bounding_box(&area) else {
            return Vec::new();
        };
        let centre = (bb.min + bb.max) / 2.0;
        let reach = bb.min.distance(bb.max) / 2.0 + q.spacing_mm;
        let mut loops = vec![area.clone()];
        loops.extend(self.hole_outlines(q.panel));
        // A line on the area's edge is that edge: not stitched again.
        let along_edge = |run: &[DVec2]| {
            run.windows(2).all(|w| match w {
                [a, b] => {
                    r3d_geom::polygon::distance_to_loop(&area, (*a + *b) / 2.0) < BOUND_CLEAR_MM
                }
                _ => true,
            })
        };
        let mut lines: Vec<Vec<DVec2>> = Vec::new();
        for dir in q.directions(panel.grain_angle_deg) {
            let across = dir.perp();
            let n = (reach / q.spacing_mm).ceil() as i64;
            for k in -n..=n {
                let mid = centre + across * (k as f64 * q.spacing_mm);
                let line = [mid - dir * reach, mid + dir * reach];
                lines.extend(
                    clip_inside(&line, &loops)
                        .into_iter()
                        .filter(|run| !along_edge(run)),
                );
            }
        }
        with_crossings(&lines)
    }
    pub fn interfacings(&self) -> impl Iterator<Item = (crate::id::DetailId, &Interfacing)> {
        self.details.iter().filter_map(|(id, d)| match d {
            Detail::Interfacing(i) => Some((*id, i)),
            _ => None,
        })
    }

    /// The outline of an interfacing on the pattern (mm), a closed loop;
    /// `None` when what it names is gone, open or not on its piece.
    pub fn interfacing_outline(&self, i: &Interfacing) -> Option<Vec<DVec2>> {
        let panel = self.panel(i.panel)?;
        match &i.area {
            InterfacingArea::Whole => self.panel_outline(i.panel),
            InterfacingArea::Inside { line } => self.closed_line_outline(i.panel, *line),
            InterfacingArea::Band { edges, width_mm } => {
                let on_piece = !edges.is_empty()
                    && edges.iter().all(|e| panel.contour.contains(e))
                    && width_mm.is_finite()
                    && *width_mm > 0.0;
                if !on_piece {
                    return None;
                }
                let mut side: Vec<DVec2> = Vec::new();
                for e in edges {
                    for p in self.edge_polyline(*e)? {
                        if side.last().is_none_or(|q| q.distance(p) > 1e-9) {
                            side.push(p);
                        }
                    }
                }
                // The edges, then back along them `width_mm` inside.
                let mut inner = offset_polyline(&side, *width_mm);
                inner.reverse();
                side.extend(inner);
                (side.len() > 3).then_some(side)
            }
        }
    }

    /// The lines that bound an interfacing inside its piece, for the
    /// mesh: its outline cut where it runs on the piece's (mm). None for
    /// a whole piece.
    pub fn interfacing_bounds(&self, i: &Interfacing) -> Vec<Vec<DVec2>> {
        let (Some(mut outline), Some(piece)) =
            (self.interfacing_outline(i), self.panel_outline(i.panel))
        else {
            return Vec::new();
        };
        if matches!(i.area, InterfacingArea::Whole) {
            return Vec::new();
        }
        if let Some(first) = outline.first().copied() {
            outline.push(first);
        }
        // Only where it runs inside the piece, 1 mm clear of its outline
        // (an edge along the piece's is the piece's own).
        let mut loops = vec![piece.clone()];
        loops.extend(self.hole_outlines(i.panel));
        clip_inside(&outline, &loops)
            .into_iter()
            .flat_map(|run| split_near(&densified(&run, BOUND_CLEAR_MM), &piece, BOUND_CLEAR_MM))
            .collect()
    }
    /// A topstitch's thread colour (`0xRRGGBB`): its own, or a darker
    /// shade of its garment's.
    pub fn thread_rgb(&self, t: &Topstitch) -> u32 {
        t.color.as_deref().and_then(parse_hex).unwrap_or_else(|| {
            let garment = self
                .side_panel(&t.side)
                .and_then(|p| self.panel(p))
                .and_then(|p| self.garments().get(&p.garment));
            darker(garment.map_or(0x808080, |g| g.rgb()))
        })
    }

    /// A topstitch's rows on the pattern (mm), each cut where it leaves
    /// its piece (or enters a hole): pieces of rows, in order.
    pub fn topstitch_rows(&self, t: &Topstitch) -> Vec<Vec<DVec2>> {
        let (Some(panel), Some(mut side)) = (self.side_panel(&t.side), self.side_polyline(&t.side))
        else {
            return Vec::new();
        };
        // Walked forward: an outline's inside is on its left.
        if t.side.reversed {
            side.reverse();
        }
        let Some(outline) = self.panel_outline(panel) else {
            return Vec::new();
        };
        let mut loops = vec![outline];
        loops.extend(self.hole_outlines(panel));
        let sign = if t.offset_mm < 0.0 { -1.0 } else { 1.0 };
        let mut out = Vec::new();
        for k in 0..t.rows {
            let by = t.offset_mm + sign * t.gap_mm * f64::from(k);
            let row = if by == 0.0 {
                side.clone()
            } else {
                offset_polyline(&side, by)
            };
            out.extend(clip_inside(&row, &loops));
        }
        out
    }

    /// An elastic's relaxed length (mm): its side's times its ratio.
    pub fn elastic_length(&self, e: &Elastic) -> Option<f64> {
        Some(self.side_length(&e.side)? * e.ratio)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn rows_run_inside_the_piece_and_stop_at_its_outline() {
        let square = vec![
            DVec2::ZERO,
            DVec2::new(400.0, 0.0),
            DVec2::new(400.0, 300.0),
            DVec2::new(0.0, 300.0),
        ];
        // The bottom edge moved 6 mm to its left: inside.
        let row = offset_polyline(&[DVec2::ZERO, DVec2::new(400.0, 0.0)], 6.0);
        assert_eq!(row, vec![DVec2::new(0.0, 6.0), DVec2::new(400.0, 6.0)]);
        let runs = clip_inside(&row, std::slice::from_ref(&square));
        assert_eq!(runs.len(), 1);
        assert!((runs[0][0] - DVec2::new(0.5, 6.0)).length() < 1e-9);
        assert!((runs[0][1] - DVec2::new(399.5, 6.0)).length() < 1e-9);
        // A hole cuts a row in two.
        let hole = vec![
            DVec2::new(100.0, 0.0),
            DVec2::new(100.0, 50.0),
            DVec2::new(200.0, 50.0),
            DVec2::new(200.0, 0.0),
        ];
        let runs = clip_inside(&row, &[square, hole]);
        assert_eq!(runs.len(), 2);
        assert!((runs[0][1].x - 99.5).abs() < 1e-9 && (runs[1][0].x - 200.5).abs() < 1e-9);
    }
}
