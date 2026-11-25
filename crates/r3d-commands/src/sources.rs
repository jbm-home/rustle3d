//! Garments imported from Seamly2D/Valentina patterns (spec 28). The
//! pattern kept in the garment is a list of drafting steps with
//! formulas: it is run again for each size's body to grade the pieces,
//! and for a new body to draft them again (`garment.redraft`). Either
//! way the pieces keep what was done to them since: each point moves as
//! the pattern moves it between the two bodies, matched by its place in
//! the outline; a piece whose outline gained or lost points is skipped.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use r3d_core::{
    Body, Edge, EdgeCurve, GarmentId, PanelId, PathSegment, PatternSource, Point, Project,
    Transaction,
};
use r3d_geom::{DVec2, polygon};
use r3d_io::import::ImportedPiece;
use r3d_io::import::val::{self, BODY_NAMES, Measurements, Values};
use rayon::prelude::*;

use crate::error::CommandError;
use crate::progress::{Progress, Status};

/// The values the pattern is run with for `body` (none: the measurement
/// file's own).
pub(crate) fn values(src: &PatternSource, body: Option<&Body>) -> Result<Values, CommandError> {
    let file = src
        .measurements
        .as_deref()
        .map(|t| Measurements::parse(t.as_bytes()))
        .transpose()?;
    Ok(val::values_for(file.as_ref(), body)?)
}

/// The pattern's pieces for `body`, by their id in the file.
pub(crate) fn run(
    src: &PatternSource,
    body: Option<&Body>,
) -> Result<HashMap<u32, ImportedPiece>, CommandError> {
    let e = val::evaluate(src.pattern.as_bytes(), &values(src, body)?)?;
    Ok(e.pieces.into_iter().collect())
}

/// What a pattern leaves out when drafted for its body: the pieces that
/// could not be built and the kinds of steps not supported.
pub(crate) fn report(src: &PatternSource) -> Result<(usize, Vec<String>), CommandError> {
    let e = val::evaluate(src.pattern.as_bytes(), &values(src, src.body.as_ref())?)?;
    Ok((e.ignored, e.unsupported))
}

/// One edge of a piece's contour: where it starts, and its curve's
/// handles.
pub(crate) type ContourEdge = (DVec2, Option<[DVec2; 2]>);

/// A piece's contour as a panel stores it: counter-clockwise, edge `j`
/// starting at point `j` (`panel.create_path` turns a clockwise outline
/// round from its last point).
fn contour(piece: &ImportedPiece) -> Vec<ContourEdge> {
    let mut edges = Vec::new();
    let mut at = piece.outline.start;
    for s in &piece.outline.segments {
        edges.push(match s {
            PathSegment::Line { .. } => (at, None),
            PathSegment::Cubic { c1, c2, .. } => (at, Some([*c1, *c2])),
        });
        at = s.end();
    }
    if polygon::signed_area(&piece.outline.polyline()) >= 0.0 {
        return edges;
    }
    let n = edges.len();
    (0..n)
        .filter_map(|j| {
            let (pos, _) = edges.get(n - 1 - j)?;
            let (_, handles) = edges.get((2 * n - 2 - j) % n)?;
            Some((*pos, handles.map(|[c1, c2]| [c2, c1])))
        })
        .collect()
}

/// The body the sizes are measured from: the avatar's, else the
/// measurement file's height, bust, waist and hips at its base.
fn reference_body(p: &Project, src: &PatternSource) -> Option<Body> {
    if let Some(a) = p.avatar() {
        return Some(a.body);
    }
    let v = values(src, None).ok()?;
    let cm = |name: &str| v.get(name).filter(|v| v.length).map(|v| v.value / 10.0);
    let [h, b, w, hp] = BODY_NAMES.map(|(n, _)| cm(n));
    Some(Body {
        masculinity: 0.0,
        height_cm: h?,
        bust_cm: b?,
        waist_cm: w?,
        hips_cm: hp?,
        age_years: None,
    })
}

/// The body of size `i`: the reference body changed by the size's
/// differences.
fn size_body(p: &Project, src: &PatternSource, i: usize) -> Option<Body> {
    let body = reference_body(p, src)?;
    let size = p.grading()?.sizes.get(i)?;
    Some(Body {
        height_cm: body.height_cm + size.height_cm,
        bust_cm: body.bust_cm + size.bust_cm,
        waist_cm: body.waist_cm + size.waist_cm,
        hips_cm: body.hips_cm + size.hips_cm,
        ..body
    })
}

/// The garment's source pattern.
fn source(p: &Project, garment: GarmentId) -> Option<PatternSource> {
    p.garments().get(&garment)?.source.clone()
}

/// Grade the pieces of `garment` from its source pattern, run for every
/// size's body: each point and curve handle moves as the pattern moves it
/// between the base size and that size. Returns whether anything was
/// graded.
pub(crate) fn grade(
    tx: &mut Transaction<'_>,
    garment: GarmentId,
    progress: &Progress,
) -> Result<bool, CommandError> {
    let p = tx.project();
    let (Some(g), Some(src)) = (p.grading().cloned(), source(p, garment)) else {
        return Ok(false);
    };
    let n = g.sizes.len();
    // Without a body to measure the sizes from, the pieces stay as they
    // are in every size.
    let Some(bodies) = (0..n)
        .map(|i| size_body(p, &src, i))
        .collect::<Option<Vec<Body>>>()
    else {
        return Ok(false);
    };
    let done = AtomicU32::new(0);
    let total = u32::try_from(n).unwrap_or(u32::MAX);
    progress.report(&Status {
        task: "pattern.sizes",
        done: 0,
        total,
        item: String::new(),
    });
    let drafts = bodies
        .par_iter()
        .zip(&g.sizes)
        .map(|(body, size)| {
            progress.check()?;
            let pieces = run(&src, Some(body))?;
            progress.report(&Status {
                task: "pattern.sizes",
                done: done.fetch_add(1, Ordering::Relaxed) + 1,
                total,
                item: size.name.clone(),
            });
            Ok(pieces)
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    let mut any = false;
    for sp in &src.pieces {
        let Some(own) = tx.project().panel(sp.panel).map(|p| p.contour.clone()) else {
            continue;
        };
        let Some(sized) = drafts
            .iter()
            .map(|d| d.get(&sp.piece).map(contour))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        if sized.iter().any(|c| c.len() != own.len()) {
            continue;
        }
        let Some(base) = sized.get(g.base) else {
            continue;
        };
        for (j, edge_id) in own.iter().enumerate() {
            let Some(edge) = tx.project().edge(*edge_id).cloned() else {
                continue;
            };
            let Some((base_start, base_handles)) = base.get(j) else {
                continue;
            };
            let mut grade = vec![DVec2::ZERO; n];
            let mut handles = vec![[DVec2::ZERO; 2]; n];
            for (i, c) in sized.iter().enumerate() {
                let Some((start, h)) = c.get(j) else {
                    continue;
                };
                if let Some(slot) = grade.get_mut(i) {
                    *slot = *start - *base_start;
                }
                if let (Some([c1, c2]), Some([b1, b2]), Some(slot)) =
                    (h, base_handles, handles.get_mut(i))
                {
                    *slot = [*c1 - *b1, *c2 - *b2];
                }
            }
            if let Some(point) = tx.project().point(edge.start).cloned() {
                let grade = if grade.iter().all(|g| *g == DVec2::ZERO) {
                    Vec::new()
                } else {
                    any = true;
                    grade
                };
                tx.put_point(edge.start, Point { grade, ..point });
            }
            let graded = matches!(edge.curve, EdgeCurve::Cubic { .. })
                && handles.iter().flatten().any(|g| *g != DVec2::ZERO);
            tx.put_edge(
                *edge_id,
                Edge {
                    grade: if graded { handles } else { Vec::new() },
                    ..edge
                },
            );
        }
    }
    Ok(any)
}

/// Move the points of `panel` as a pattern moves its outline from `a` to
/// `b` (each edge's start and handles, in outline order, from the same
/// origin); its curves' handles, internal lines and pins follow. `false`
/// when the piece is gone or its outline does not match the pattern's.
pub(crate) fn move_piece(
    tx: &mut Transaction<'_>,
    panel: PanelId,
    a: &[ContourEdge],
    b: &[ContourEdge],
) -> bool {
    let Some(own) = tx.project().panel(panel).map(|p| p.contour.clone()) else {
        return false;
    };
    if a.len() != own.len() || b.len() != own.len() {
        return false;
    }
    let mut moves = Vec::new();
    for (j, edge_id) in own.iter().enumerate() {
        let (Some(edge), Some((a0, ah)), Some((b0, bh))) =
            (tx.project().edge(*edge_id).cloned(), a.get(j), b.get(j))
        else {
            continue;
        };
        let d = *b0 - *a0;
        if let Some(point) = tx.project().point(edge.start).cloned() {
            moves.push((point.pos, d));
            tx.put_point(
                edge.start,
                Point {
                    pos: point.pos + d,
                    ..point
                },
            );
        }
        if let EdgeCurve::Cubic { c1, c2 } = edge.curve {
            // The handles move as the pattern's do; on an edge curved
            // here, with its ends.
            let next = (j + 1) % own.len();
            let end_d = match (a.get(next), b.get(next)) {
                (Some((a1, _)), Some((b1, _))) => *b1 - *a1,
                _ => d,
            };
            let (d1, d2) = match (ah, bh) {
                (Some([a1, a2]), Some([b1, b2])) => (*b1 - *a1, *b2 - *a2),
                _ => (d, end_d),
            };
            tx.put_edge(
                *edge_id,
                Edge {
                    curve: EdgeCurve::Cubic {
                        c1: c1 + d1,
                        c2: c2 + d2,
                    },
                    ..edge
                },
            );
        }
    }
    let lines: Vec<_> = tx
        .project()
        .internal_lines()
        .iter()
        .filter(|(_, l)| l.panel == panel)
        .map(|(id, l)| (*id, l.clone()))
        .collect();
    for (id, mut l) in lines {
        l.start = moved(&moves, l.start);
        for s in &mut l.segments {
            *s = match s {
                PathSegment::Line { to } => PathSegment::Line {
                    to: moved(&moves, *to),
                },
                PathSegment::Cubic { c1, c2, to } => PathSegment::Cubic {
                    c1: moved(&moves, *c1),
                    c2: moved(&moves, *c2),
                    to: moved(&moves, *to),
                },
            };
        }
        tx.put_internal_line(id, l);
    }
    let pins: Vec<_> = tx
        .project()
        .pins()
        .iter()
        .filter(|(_, p)| p.panel == panel)
        .map(|(id, p)| (*id, *p))
        .collect();
    for (id, mut pin) in pins {
        pin.at = moved(&moves, pin.at);
        tx.put_pin(id, pin);
    }
    // Buttons and buttonholes on the piece follow it too.
    let buttons: Vec<_> = tx
        .project()
        .buttons()
        .iter()
        .filter(|(_, b)| b.panel == panel || b.hole.panel == panel)
        .map(|(id, b)| (*id, b.clone()))
        .collect();
    for (id, mut b) in buttons {
        if b.panel == panel {
            b.at = moved(&moves, b.at);
        }
        if b.hole.panel == panel {
            b.hole.at = moved(&moves, b.hole.at);
        }
        tx.put_button(id, b);
    }
    true
}

/// How far each point of a piece moved; inside the piece, by inverse
/// squared distance to them (as internal lines follow grades).
fn moved(moves: &[(DVec2, DVec2)], p: DVec2) -> DVec2 {
    let mut sum = DVec2::ZERO;
    let mut weights = 0.0;
    for (at, d) in moves {
        let w = 1.0 / (at.distance_squared(p) + 1.0);
        sum += *d * w;
        weights += w;
    }
    p + if weights > 0.0 { sum / weights } else { sum }
}

/// Draft the pieces of `garment` again for `body`: each point and curve
/// handle moves as the pattern moves it from the body it was drafted for
/// to this one; internal lines and pins follow the points around them.
/// The garment remembers the new body, and is graded again in a size
/// range. Returns the panels skipped (their outline changed since).
pub(crate) fn redraft(
    tx: &mut Transaction<'_>,
    garment: GarmentId,
    body: Body,
    progress: &Progress,
) -> Result<Vec<r3d_core::PanelId>, CommandError> {
    let Some(src) = source(tx.project(), garment) else {
        return Err(CommandError::NoSource {
            garment: garment.into(),
        });
    };
    let old = run(&src, src.body.as_ref())?;
    let new = run(&src, Some(&body))?;
    let mut skipped = Vec::new();
    for sp in &src.pieces {
        let moved = match (old.get(&sp.piece), new.get(&sp.piece)) {
            (Some(a), Some(b)) => move_piece(tx, sp.panel, &contour(a), &contour(b)),
            _ => false,
        };
        if !moved && tx.project().panel(sp.panel).is_some() {
            skipped.push(sp.panel);
        }
    }
    if let Some(g) = tx.project().garments().get(&garment).cloned() {
        tx.put_garment(
            garment,
            r3d_core::Garment {
                source: Some(PatternSource {
                    body: Some(body),
                    ..src
                }),
                ..g
            },
        );
    }
    grade(tx, garment, progress)?;
    Ok(skipped)
}
