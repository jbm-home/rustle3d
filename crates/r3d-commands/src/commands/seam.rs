use r3d_core::seam::{Span, Track};
use r3d_core::{AnyId, EdgeId, InternalLineId, Project, Seam, SeamId, SeamSide, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::mirror::mirror_twin;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// One side of a new seam: a whole edge, a run of edges, or an internal
/// line; all of it or part of it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSideSpec {
    /// One edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<EdgeId>,
    /// Or consecutive outline edges of one piece, in their order around it
    /// (e.g. a pocket's sides and bottom).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<EdgeId>,
    /// Or an internal line (a pocket's placement line, a tuck's line).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<InternalLineId>,
    /// Only from here along the path (mm from its start). Omitted: its
    /// start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_mm: Option<f64>,
    /// Only to here (mm from the path's start). Omitted: its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_mm: Option<f64>,
    /// Walk the side from its end instead of its start. Omitted: `false`
    /// for side `a`; for side `b`, an edge sewn to its linked mirror image
    /// meets it mirrored point to mirrored point (a centre seam), other
    /// sides are walked both the same way on the pattern (what sewing two
    /// panels laid side by side needs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversed: Option<bool>,
}

impl SeamSideSpec {
    /// The side it names (walked forward), checked: one path, existing,
    /// its edges consecutive on one piece's outline, its range inside it.
    pub(crate) fn side(&self, field: &'static str, p: &Project) -> Result<SeamSide, CommandError> {
        let edges = match (self.edge, self.edges.is_empty(), self.line) {
            (Some(e), true, None) => vec![e],
            (None, false, None) => self.edges.clone(),
            (None, true, Some(_)) => Vec::new(),
            _ => return Err(CommandError::invalid(field, Invalid::Format)),
        };
        for e in &edges {
            p.edge(*e).ok_or_else(|| CommandError::not_found(*e))?;
        }
        if let Some(l) = self.line {
            p.internal_line(l)
                .ok_or_else(|| CommandError::not_found(l))?;
        }
        finite(field, &[self.from_mm, self.to_mm].map(|v| v.unwrap_or(0.0)))?;
        let mut side = SeamSide {
            edges,
            line: self.line,
            from_mm: self.from_mm,
            to_mm: self.to_mm,
            reversed: false,
        };
        // Joined edges of one piece, a range within the path.
        let spans = p
            .side_spans(&side)
            .ok_or_else(|| CommandError::invalid(field, Invalid::NotJoined))?;
        let joined = side.edges.windows(2).all(|w| match w {
            [a, b] => p
                .edge(*a)
                .zip(p.edge(*b))
                .is_some_and(|(a, b)| a.end == b.start && a.panel == b.panel),
            _ => true,
        });
        if !joined {
            return Err(CommandError::invalid(field, Invalid::NotJoined));
        }
        let total: f64 = match side.line {
            Some(l) => p.track_length(Track::Line(l)).unwrap_or_default(),
            None => side
                .edges
                .iter()
                .map(|e| p.edge_length(*e).unwrap_or_default())
                .sum(),
        };
        let from = self.from_mm.unwrap_or(0.0);
        let mut to = self.to_mm.unwrap_or(total);
        // A length typed as shown (a curve's measured length rounds to
        // it) is the path's end.
        if (to - total).abs() <= END_TOLERANCE_MM {
            to = total;
            side.to_mm = None;
        }
        if !(from >= 0.0 && from < to && to <= total) {
            return Err(CommandError::invalid(
                field,
                Invalid::Range(0.0, (total * 1000.0).round() / 1000.0),
            ));
        }
        if spans.iter().all(|s| s.length() <= 0.0) {
            return Err(CommandError::invalid(field, Invalid::Empty));
        }
        Ok(side)
    }
}

/// How far past a path's length a range's end may be given and still mean
/// its end (mm).
const END_TOLERANCE_MM: f64 = 0.05;

/// Whether two stretches of the same track overlap (more than touching).
pub(crate) fn overlap(a: &Span, b: &Span) -> bool {
    a.track == b.track && a.from.max(b.from) < a.to.min(b.to) - 1e-6
}

/// The direction a side runs on the pattern: from its first point to its
/// last.
pub(crate) fn heading(p: &Project, side: &SeamSide) -> r3d_geom::DVec2 {
    p.side_polyline(side)
        .and_then(|pts| Some(*pts.last()? - *pts.first()?))
        .unwrap_or_default()
}

/// Sew two sides together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamAdd {
    pub a: SeamSideSpec,
    pub b: SeamSideSpec,
    /// How much longer side `a` is meant to be than `b` (mm, negative:
    /// shorter), eased in: not reported as a mismatch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease_mm: Option<f64>,
}

impl Described for SeamAdd {
    const SUMMARY: &'static str = "Sew two sides together. A side is a whole `edge`, a run of consecutive outline `edges` of one piece (in order around it), or an internal `line`, each optionally only `from_mm`/`to_mm` along it (a seam between two notches, a pocket sewn on its placement line, the two halves of a pleat on one edge). Points are matched along the sides, start to start unless a side is reversed; leave b.reversed out to have it chosen (an edge and its linked mirror image meet mirrored point to mirrored point; other sides are walked both the same way on the pattern), and use seam.flip if the seam looks twisted. Stretches already sewn cannot be sewn again (invalid.overlaps). `ease_mm` is how much longer side a is meant to be. Returns the seam.";
}

impl CommandSpec for SeamAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let mut a = self.a.side("a", p)?;
        let mut b = self.b.side("b", p)?;
        let ease = self.ease_mm.unwrap_or(0.0);
        finite("ease_mm", &[ease])?;
        // Nothing sewn twice: not against another seam, nor a side on
        // itself.
        let (sa, sb) = (
            p.side_spans(&a).unwrap_or_default(),
            p.side_spans(&b).unwrap_or_default(),
        );
        free_to_join(p, &sa, &sb)?;
        a.reversed = self.a.reversed.unwrap_or(false);
        b.reversed = self
            .b
            .reversed
            .unwrap_or_else(|| default_reversed(p, &a, &b));
        let id = tx.new_seam_id();
        tx.put_seam(
            id,
            Seam {
                a,
                b,
                gathered: false,
                ease_mm: ease,
            },
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Every stretch already joined, by a seam or a zipper, with what joins
/// it.
fn joined(p: &Project) -> Vec<(AnyId, Span)> {
    let seams = p
        .seams()
        .iter()
        .flat_map(|(id, s)| [(AnyId::from(*id), &s.a), (AnyId::from(*id), &s.b)]);
    let zippers = p
        .zippers()
        .iter()
        .flat_map(|(id, z)| [(AnyId::from(*id), &z.a), (AnyId::from(*id), &z.b)]);
    seams
        .chain(zippers)
        .flat_map(|(id, s)| {
            p.side_spans(s)
                .unwrap_or_default()
                .into_iter()
                .map(move |span| (id, span))
        })
        .collect()
}

/// Two sides about to be joined (by a seam or a zipper), as stretches:
/// nothing joined twice, not against another seam or zipper, nor a side
/// on itself.
pub(crate) fn free_to_join(p: &Project, sa: &[Span], sb: &[Span]) -> Result<(), CommandError> {
    let taken = joined(p);
    for (field, mine) in [("a", sa), ("b", sb)] {
        for x in mine.iter() {
            if let Some((by, y)) = taken.iter().find(|(_, y)| overlap(x, y)) {
                // An edge already joined there: say by what.
                return Err(match y.track {
                    Track::Edge(edge) => CommandError::Sewn {
                        edge: edge.into(),
                        seam: *by,
                    },
                    Track::Line(_) => CommandError::invalid(field, Invalid::Overlaps),
                });
            }
        }
    }
    if sa.iter().any(|x| sb.iter().any(|y| overlap(x, y))) {
        return Err(CommandError::invalid("b", Invalid::SamePoint));
    }
    Ok(())
}

/// Whether side `b` is walked reversed when not said: an edge joined to
/// its linked mirror image meets it mirrored point to mirrored point;
/// other sides are walked both the same way on the pattern.
pub(crate) fn default_reversed(p: &Project, a: &SeamSide, b: &SeamSide) -> bool {
    match (a.whole_edge(), b.whole_edge()) {
        (Some(ea), Some(eb))
            if mirror_twin(p, ea).is_some_and(|(t, _)| t == eb)
                || mirror_twin(p, eb).is_some_and(|(t, _)| t == ea) =>
        {
            // A reflection reverses a contour: an edge's start mirrors
            // its twin's end.
            !a.reversed
        }
        _ => heading(p, b).dot(heading(p, a)) < 0.0,
    }
}

fn get_seam(tx: &Transaction<'_>, id: SeamId) -> Result<Seam, CommandError> {
    tx.project()
        .seams()
        .get(&id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

/// Set how much longer one side of a seam is meant to be.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSetEase {
    pub seam: SeamId,
    /// How much longer side `a` is meant to be than side `b` (mm;
    /// negative, shorter): eased in along the seam (a sleeve cap, a back
    /// shoulder), not reported as a mismatch.
    pub mm: f64,
}

impl Described for SeamSetEase {
    const SUMMARY: &'static str = "Set a seam's ease: how much longer its side a is meant to be than side b (mm; negative, shorter). The extra length is eased in along the seam when it drapes; only the difference beyond the ease is reported as a mismatch.";
}

impl CommandSpec for SeamSetEase {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("mm", &[self.mm])?;
        let seam = get_seam(tx, self.seam)?;
        tx.put_seam(
            self.seam,
            Seam {
                ease_mm: self.mm,
                ..seam
            },
        );
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("seam.set_ease:{}", self.seam))
    }
}

/// Unpick a seam.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamRemove {
    pub seam: SeamId,
}

impl Described for SeamRemove {
    const SUMMARY: &'static str = "Remove a seam (the edges are no longer sewn).";
}

impl CommandSpec for SeamRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_seam(tx, self.seam)?;
        tx.remove_seam(self.seam);
        Ok(CommandOutput::default())
    }
}

/// Flip the direction of a seam's second side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamFlip {
    pub seam: SeamId,
}

impl Described for SeamFlip {
    const SUMMARY: &'static str =
        "Reverse how a seam matches its two edges (fixes a twisted seam).";
}

/// Gather a seam's longer side onto the shorter one, or stop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSetGathered {
    pub seam: SeamId,
    pub gathered: bool,
}

impl Described for SeamSetGathered {
    const SUMMARY: &'static str = "Mark a seam as gathered (or not): its longer side is gathered evenly onto the shorter one when it drapes (a gathered skirt into a waistband, a sleeve cap), so the difference in length is intended and not reported as a mismatch. `seam.list` gives each seam's gathering ratio.";
}

impl CommandSpec for SeamSetGathered {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let seam = get_seam(tx, self.seam)?;
        tx.put_seam(
            self.seam,
            Seam {
                gathered: self.gathered,
                ..seam
            },
        );
        Ok(CommandOutput::default())
    }
}

impl CommandSpec for SeamFlip {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let mut seam = get_seam(tx, self.seam)?;
        seam.b.reversed = !seam.b.reversed;
        tx.put_seam(self.seam, seam);
        Ok(CommandOutput::default())
    }
}
