//! Nesting pattern pieces on a strip of fabric (a marker).
//!
//! Raster bottom-left-fill. The strip (fixed width along y, open length
//! along x) and each piece, in each of its allowed orientations, are
//! rasterised on a square grid; a cell is taken when the piece touches
//! it, so two pieces whose rasters do not share a cell never overlap
//! (Bennell & Oliveira, "The geometry of nesting problems: a tutorial",
//! EJOR 184, 2008, §3: raster methods; Oliveira & Ferreira, "Algorithms
//! for nesting problems", 1993). Pieces are placed one by one, largest
//! first, each at the bottom-left-fill position (Jakobs, "On genetic
//! algorithms for the packing of polygons", EJOR 88, 1996; Burke,
//! Hellier, Kendall & Whitwell, "A new bottom-left-fill heuristic
//! algorithm for the two-dimensional irregular packing problem",
//! Operations Research 54(3), 2006): scanning the length from the start
//! and the width from the bottom, the first place where it fits, so
//! holes left by earlier pieces are filled. Among a piece's
//! orientations, the one whose right end is leftmost wins (then the
//! lowest). Deterministic: no randomness, ties broken by order.
//!
//! Units: whatever the caller uses (millimetres in Rustle3D).

use glam::DVec2;

use crate::polygon;

/// How a piece is laid: turned by `angle` (radians, counter-clockwise),
/// then, when `mirrored`, turned over across the length (y → −y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orientation {
    pub angle: f64,
    pub mirrored: bool,
}

impl Orientation {
    /// Where point `p` of the piece goes, before the placement's offset.
    pub fn apply(&self, p: DVec2) -> DVec2 {
        let q = DVec2::from_angle(self.angle).rotate(p);
        if self.mirrored {
            DVec2::new(q.x, -q.y)
        } else {
            q
        }
    }
}

/// A piece to place: its outline (a closed polygon) and the orientations
/// it may take.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub outline: Vec<DVec2>,
    pub orientations: Vec<Orientation>,
}

/// Where a piece went: a point `p` of its outline is at
/// `orientations[orientation].apply(p) + offset` on the strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub orientation: usize,
    pub offset: DVec2,
}

/// A finished layout: one placement per piece, in the pieces' order, and
/// the strip length used (the rightmost point of any piece).
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub placed: Vec<Placed>,
    pub length: f64,
}

/// Why pieces could not be nested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NestError {
    /// Piece `0` is wider than the strip in every orientation it may take
    /// (or has none, or no area).
    TooWide(usize),
    /// The width or the cell size is not a positive finite number, or the
    /// grid would be too large (more than [`MAX_ROWS`] cells across).
    BadGrid,
}

/// Most cells across the strip (a 3 m wide fabric at 1 mm).
pub const MAX_ROWS: usize = 3000;

/// A piece in one orientation, rasterised: column `c` of the raster is
/// `words` 64-bit words of rows, bit `r` set when the piece touches cell
/// `(c, r)`.
struct Raster {
    cols: usize,
    rows: usize,
    words: usize,
    bits: Vec<u64>,
    /// The oriented outline's bounding-box corner, at cell (0, 0).
    min: DVec2,
    max_x: f64,
}

impl Raster {
    fn column(&self, c: usize) -> &[u64] {
        self.bits
            .get(c * self.words..(c + 1) * self.words)
            .unwrap_or(&[])
    }
}

fn set(bits: &mut [u64], words: usize, c: usize, r: usize) {
    if let Some(w) = bits.get_mut(c * words + r / 64) {
        *w |= 1 << (r % 64);
    }
}

/// The cells `outline` touches, on a grid of `cell` whose origin is the
/// outline's bounding-box corner: every cell an edge crosses, and every
/// cell whose centre is inside.
fn rasterise(outline: &[DVec2], cell: f64) -> Option<Raster> {
    let bb = polygon::bounding_box(outline)?;
    let size = bb.max - bb.min;
    let cols = ((size.x / cell).ceil() as usize).max(1);
    let rows = ((size.y / cell).ceil() as usize).max(1);
    if rows > MAX_ROWS || cols > 64 * MAX_ROWS {
        return None;
    }
    let words = rows.div_ceil(64);
    let mut bits = vec![0u64; cols * words];
    let local: Vec<DVec2> = outline.iter().map(|p| (*p - bb.min) / cell).collect();
    let clamp_c = |x: f64| (x.floor().max(0.0) as usize).min(cols - 1);
    let clamp_r = |y: f64| (y.floor().max(0.0) as usize).min(rows - 1);
    let n = local.len();
    // Edges: for each column an edge spans, the rows of its stretch there.
    for i in 0..n {
        let (a, b) = (local[i], local[(i + 1) % n]);
        let (lo, hi) = if a.x <= b.x { (a, b) } else { (b, a) };
        for c in clamp_c(lo.x)..=clamp_c(hi.x) {
            let (x0, x1) = ((c as f64).max(lo.x), ((c + 1) as f64).min(hi.x));
            let y_at = |x: f64| {
                if hi.x - lo.x > 1e-12 {
                    lo.y + (hi.y - lo.y) * ((x - lo.x) / (hi.x - lo.x)).clamp(0.0, 1.0)
                } else {
                    lo.y
                }
            };
            let (ya, yb) = if hi.x - lo.x > 1e-12 {
                (y_at(x0), y_at(x1))
            } else {
                (lo.y, hi.y)
            };
            for r in clamp_r(ya.min(yb))..=clamp_r(ya.max(yb)) {
                set(&mut bits, words, c, r);
            }
        }
    }
    // Inside: cells whose centre is in the outline (even-odd), by column.
    let mut crossings = Vec::new();
    for c in 0..cols {
        let x = c as f64 + 0.5;
        crossings.clear();
        for i in 0..n {
            let (a, b) = (local[i], local[(i + 1) % n]);
            if (a.x > x) != (b.x > x) {
                crossings.push(a.y + (x - a.x) / (b.x - a.x) * (b.y - a.y));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for [enter, leave] in crossings.as_chunks::<2>().0 {
            // Rows whose centre r + 0.5 lies in [enter, leave].
            let first = (enter - 0.5).ceil().max(0.0) as usize;
            let last = (leave - 0.5).floor();
            if last < 0.0 {
                continue;
            }
            for r in first..=(last as usize).min(rows - 1) {
                set(&mut bits, words, c, r);
            }
        }
    }
    Some(Raster {
        cols,
        rows,
        words,
        bits,
        min: bb.min,
        max_x: bb.max.x,
    })
}

/// The strip: one column of `words` words per cell along the length,
/// growing as pieces are placed.
struct Strip {
    rows: usize,
    words: usize,
    bits: Vec<u64>,
}

impl Strip {
    fn cols(&self) -> usize {
        self.bits.len() / self.words
    }

    /// Whether `piece` fits with its cell (0, 0) at strip cell `(x, y)`.
    fn fits(&self, piece: &Raster, x: usize, y: usize) -> bool {
        if y + piece.rows > self.rows {
            return false;
        }
        let (shift_words, shift_bits) = (y / 64, (y % 64) as u32);
        for c in 0..piece.cols {
            let Some(strip) = self
                .bits
                .get((x + c) * self.words..(x + c + 1) * self.words)
            else {
                // Past the strip's end: empty.
                return true;
            };
            let col = piece.column(c);
            // Word k of the piece's column shifted up by y rows.
            for k in shift_words..self.words {
                let src = k - shift_words;
                let low = col.get(src).copied().unwrap_or(0);
                let carry = if shift_bits > 0 && src > 0 {
                    col.get(src - 1).copied().unwrap_or(0) >> (64 - shift_bits)
                } else {
                    0
                };
                let shifted = (low << shift_bits) | carry;
                if shifted & strip.get(k).copied().unwrap_or(0) != 0 {
                    return false;
                }
                if src > piece.words {
                    break;
                }
            }
        }
        true
    }

    fn put(&mut self, piece: &Raster, x: usize, y: usize) {
        let needed = (x + piece.cols) * self.words;
        if self.bits.len() < needed {
            self.bits.resize(needed, 0);
        }
        for c in 0..piece.cols {
            for r in 0..piece.rows {
                let bit = piece
                    .column(c)
                    .get(r / 64)
                    .is_some_and(|w| w & (1 << (r % 64)) != 0);
                if bit {
                    set(&mut self.bits, self.words, x + c, y + r);
                }
            }
        }
    }

    /// The bottom-left-fill position of `piece`: the first `(x, y)` where
    /// it fits, scanning x from 0, then y from 0; none at or past
    /// `stop_x` (a better orientation already ends there).
    fn first_fit(&self, piece: &Raster, stop_x: usize) -> Option<(usize, usize)> {
        if piece.rows > self.rows {
            return None;
        }
        // At the strip's end, every piece fits at the bottom.
        let last = self.cols().min(stop_x);
        for x in 0..=last {
            if x >= stop_x {
                return None;
            }
            for y in 0..=(self.rows - piece.rows) {
                if self.fits(piece, x, y) {
                    return Some((x, y));
                }
            }
        }
        None
    }
}

/// Nest `pieces` on a strip `width` wide with cells of `cell`.
pub fn nest(pieces: &[Piece], width: f64, cell: f64) -> Result<Layout, NestError> {
    if !(width.is_finite() && cell.is_finite() && width > 0.0 && cell > 0.0) {
        return Err(NestError::BadGrid);
    }
    let rows = (width / cell + 1e-9).floor() as usize;
    if rows == 0 || rows > MAX_ROWS {
        return Err(NestError::BadGrid);
    }
    // Every orientation of every piece, rasterised.
    let rasters: Vec<Vec<Option<Raster>>> = pieces
        .iter()
        .map(|p| {
            p.orientations
                .iter()
                .map(|o| {
                    let turned: Vec<DVec2> = p.outline.iter().map(|q| o.apply(*q)).collect();
                    rasterise(&turned, cell)
                })
                .collect()
        })
        .collect();
    // Largest first; ties in the given order.
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    let area = |i: usize| {
        pieces
            .get(i)
            .map_or(0.0, |p| polygon::signed_area(&p.outline).abs())
    };
    order.sort_by(|a, b| area(*b).total_cmp(&area(*a)).then(a.cmp(b)));

    let words = rows.div_ceil(64);
    let mut strip = Strip {
        rows,
        words,
        bits: Vec::new(),
    };
    let mut placed = vec![
        Placed {
            orientation: 0,
            offset: DVec2::ZERO,
        };
        pieces.len()
    ];
    let mut length: f64 = 0.0;
    for i in order {
        let options = rasters.get(i).ok_or(NestError::TooWide(i))?;
        // (right end, x, y, orientation)
        let mut best: Option<(usize, usize, usize, usize)> = None;
        for (o, raster) in options.iter().enumerate() {
            let Some(raster) = raster else { continue };
            let stop = best.map_or(usize::MAX, |b| b.0.saturating_sub(raster.cols) + 1);
            if let Some((x, y)) = strip.first_fit(raster, stop) {
                let candidate = (x + raster.cols, x, y, o);
                if best.is_none_or(|b| (candidate.0, candidate.1, candidate.2) < (b.0, b.1, b.2)) {
                    best = Some(candidate);
                }
            }
        }
        let (_, x, y, o) = best.ok_or(NestError::TooWide(i))?;
        let raster = options
            .get(o)
            .and_then(Option::as_ref)
            .ok_or(NestError::TooWide(i))?;
        strip.put(raster, x, y);
        let offset = DVec2::new(x as f64, y as f64) * cell - raster.min;
        length = length.max(raster.max_x + offset.x);
        if let Some(slot) = placed.get_mut(i) {
            *slot = Placed {
                orientation: o,
                offset,
            };
        }
    }
    Ok(Layout { placed, length })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::f64::consts::PI;

    fn rect(w: f64, h: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(w, 0.0),
            DVec2::new(w, h),
            DVec2::new(0.0, h),
        ]
    }

    fn upright() -> Vec<Orientation> {
        vec![
            Orientation {
                angle: 0.0,
                mirrored: false,
            },
            Orientation {
                angle: PI,
                mirrored: false,
            },
        ]
    }

    fn laid(p: &Piece, at: &Placed) -> Vec<DVec2> {
        p.outline
            .iter()
            .map(|q| p.orientations[at.orientation].apply(*q) + at.offset)
            .collect()
    }

    /// Area shared by two laid pieces, sampled on a fine grid.
    fn overlap(a: &[DVec2], b: &[DVec2], fine: f64) -> f64 {
        let (Some(ba), Some(bb)) = (polygon::bounding_box(a), polygon::bounding_box(b)) else {
            return 0.0;
        };
        let lo = ba.min.max(bb.min);
        let hi = ba.max.min(bb.max);
        let mut shared = 0.0;
        let mut x = lo.x + fine / 2.0;
        while x < hi.x {
            let mut y = lo.y + fine / 2.0;
            while y < hi.y {
                let p = DVec2::new(x, y);
                if polygon::contains(a, p) && polygon::contains(b, p) {
                    shared += fine * fine;
                }
                y += fine;
            }
            x += fine;
        }
        shared
    }

    #[test]
    fn rectangles_fill_the_width_then_the_length() {
        let pieces: Vec<Piece> = (0..4)
            .map(|_| Piece {
                outline: rect(100.0, 50.0),
                orientations: upright(),
            })
            .collect();
        let layout = nest(&pieces, 100.0, 5.0).unwrap();
        // Two across, two along.
        assert!((layout.length - 200.0).abs() < 1e-9, "{}", layout.length);
    }

    #[test]
    fn a_hole_left_behind_is_filled() {
        // A big piece leaves a gap above it that a small one fills.
        let pieces = vec![
            Piece {
                outline: rect(300.0, 80.0),
                orientations: upright(),
            },
            Piece {
                outline: rect(100.0, 20.0),
                orientations: upright(),
            },
        ];
        let layout = nest(&pieces, 100.0, 5.0).unwrap();
        assert!((layout.length - 300.0).abs() < 1e-9);
        assert!(layout.placed[1].offset.y >= 80.0 - 1e-9);
    }

    #[test]
    fn too_wide_is_refused() {
        let pieces = vec![Piece {
            outline: rect(50.0, 200.0),
            orientations: upright(),
        }];
        assert_eq!(nest(&pieces, 100.0, 5.0), Err(NestError::TooWide(0)));
        let turned = vec![Piece {
            outline: rect(50.0, 200.0),
            orientations: vec![Orientation {
                angle: PI / 2.0,
                mirrored: false,
            }],
        }];
        assert!(nest(&turned, 100.0, 5.0).is_ok());
    }

    #[test]
    fn mirrored_keeps_the_length_direction() {
        let o = Orientation {
            angle: 0.0,
            mirrored: true,
        };
        assert_eq!(o.apply(DVec2::new(3.0, 2.0)), DVec2::new(3.0, -2.0));
    }

    fn polygon_strategy() -> impl Strategy<Value = Vec<DVec2>> {
        // A star-shaped polygon around the origin: always simple.
        prop::collection::vec(20.0..120.0f64, 3..9).prop_map(|radii| {
            let n = radii.len();
            radii
                .iter()
                .enumerate()
                .map(|(i, r)| DVec2::from_angle(2.0 * PI * i as f64 / n as f64) * *r)
                .collect()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]
        #[test]
        fn never_overlaps_and_stays_on_the_fabric(
            outlines in prop::collection::vec(polygon_strategy(), 1..7),
            turn in 0.0..(2.0 * PI),
        ) {
            let pieces: Vec<Piece> = outlines
                .into_iter()
                .map(|outline| Piece {
                    outline,
                    orientations: vec![
                        Orientation { angle: turn, mirrored: false },
                        Orientation { angle: turn + PI, mirrored: true },
                    ],
                })
                .collect();
            let width = 300.0;
            let layout = nest(&pieces, width, 4.0).unwrap();
            let laid: Vec<Vec<DVec2>> = pieces
                .iter()
                .zip(&layout.placed)
                .map(|(p, at)| laid(p, at))
                .collect();
            for l in &laid {
                let bb = polygon::bounding_box(l).unwrap();
                prop_assert!(bb.min.x >= -1e-9 && bb.min.y >= -1e-9);
                prop_assert!(bb.max.y <= width + 1e-9);
                prop_assert!(bb.max.x <= layout.length + 1e-9);
            }
            for i in 0..laid.len() {
                for j in i + 1..laid.len() {
                    prop_assert!(overlap(&laid[i], &laid[j], 1.0) == 0.0);
                }
            }
            // Deterministic.
            prop_assert_eq!(nest(&pieces, width, 4.0).unwrap(), layout);
        }
    }
}
