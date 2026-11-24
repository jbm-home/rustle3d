//! Construction details that change the cloth where they are
//! (`docs/specs/31-construction-details-2.md`): elastic, topstitching.

use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::PanelId;
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
