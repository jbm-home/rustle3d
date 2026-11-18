//! Semantic copy and paste of pieces: `panel.copy` gives a clip of pieces
//! with what belongs to them (their points, edges, notches, internal
//! lines, pins, grade rules, the seams between them and the mirror links
//! among them); `panel.paste` adds a clip to a project, in the same
//! project or another one, under fresh IDs.

use std::collections::BTreeMap;

use r3d_core::file::SCHEMA_VERSION;
use r3d_core::{
    Constraint, Edge, EdgeCurve, EdgeId, GarmentId, InternalLine, Notch, Panel, PanelId,
    PathSegment, Pin, Point, PointId, Project, Seam, SeamSide, Symmetry, Transaction,
};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Vec2, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described, QuerySpec, to_value};

/// What a clip says it is.
const FORMAT: &str = "rustle3d.clip";

/// Gap left between the pieces on the board and pasted ones (mm).
const PASTE_GAP_MM: f64 = 60.0;

/// Pieces copied with what belongs to them, under the IDs of the project
/// they come from. Serialized with the document's schema, so a clip only
/// pastes into the same version of the app.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Clip {
    format: String,
    version: u32,
    /// The size range's number of sizes and base size, if graded: grade
    /// rules paste only into the same layout.
    sizes: Option<(usize, usize)>,
    panels: BTreeMap<PanelId, Panel>,
    points: BTreeMap<PointId, Point>,
    edges: BTreeMap<EdgeId, Edge>,
    notches: Vec<Notch>,
    internal_lines: Vec<InternalLine>,
    seams: Vec<Seam>,
    pins: Vec<Pin>,
    /// Constraints whose points and edges are all in the clip.
    constraints: Vec<Constraint>,
}

/// Copy pieces: a clip to give to panel.paste.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCopy {
    /// The pieces to copy (at least one).
    pub panels: Vec<PanelId>,
}

impl Described for PanelCopy {
    const SUMMARY: &'static str = "Copy pieces into a clip (JSON) for panel.paste, in this project or another one: the pieces with their points, curves, notches, internal lines, pins and grade rules, the seams between them and the mirror links among them. Seams and links to pieces left out are not copied.";
}

impl QuerySpec for PanelCopy {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        if self.panels.is_empty() {
            return Err(CommandError::invalid("panels", Invalid::Empty));
        }
        let mut panels = BTreeMap::new();
        for id in &self.panels {
            let panel = p.panel(*id).ok_or_else(|| CommandError::not_found(*id))?;
            panels.insert(*id, panel.clone());
        }
        let points: BTreeMap<_, _> = p
            .points()
            .iter()
            .filter(|(_, pt)| panels.contains_key(&pt.panel))
            .map(|(id, pt)| (*id, pt.clone()))
            .collect();
        let edges: BTreeMap<_, _> = p
            .edges()
            .iter()
            .filter(|(_, e)| panels.contains_key(&e.panel))
            .map(|(id, e)| (*id, e.clone()))
            .collect();
        let clip = Clip {
            format: FORMAT.into(),
            version: SCHEMA_VERSION,
            sizes: p.grading().map(|g| (g.sizes.len(), g.base)),
            notches: p
                .notches()
                .values()
                .filter(|n| edges.contains_key(&n.edge))
                .cloned()
                .collect(),
            internal_lines: p
                .internal_lines()
                .values()
                .filter(|l| panels.contains_key(&l.panel))
                .cloned()
                .collect(),
            seams: p
                .seams()
                .values()
                .filter(|s| edges.contains_key(&s.a.edge) && edges.contains_key(&s.b.edge))
                .copied()
                .collect(),
            pins: p
                .pins()
                .values()
                .filter(|pin| panels.contains_key(&pin.panel))
                .copied()
                .collect(),
            constraints: p
                .constraints()
                .values()
                .filter(|c| {
                    c.points().iter().all(|pt| points.contains_key(pt))
                        && c.edges().iter().all(|e| edges.contains_key(e))
                })
                .cloned()
                .collect(),
            panels,
            points,
            edges,
        };
        to_value(&clip)
    }
}

/// Paste pieces copied with panel.copy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelPaste {
    /// The clip, as panel.copy returns it.
    pub clip: Value,
    /// How far to move the pieces on the board (mm). Omitted: to the right
    /// of the project's pieces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<Vec2>,
    /// The garment the pieces join. Omitted: the one they were copied
    /// from if this project has it, otherwise the project's first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
}

impl Described for PanelPaste {
    const SUMMARY: &'static str = "Paste pieces copied with panel.copy, under new IDs, with the seams between them, their notches, internal lines, grade rules (when the project has the same number of sizes and base size), pins and mirror links. A piece keeps its place around the body unless a piece of the project already starts there (pasted beside its original): then it is left unplaced, with no pins. Returns the new panels, then the new seams.";
}

/// IDs of the clip mapped to fresh ones.
struct Fresh<K>(BTreeMap<K, K>);

impl<K: Ord + Copy + Into<r3d_core::AnyId>> Fresh<K> {
    fn get(&self, id: K) -> Result<K, CommandError> {
        self.0
            .get(&id)
            .copied()
            .ok_or_else(|| malformed(format!("{} is not in the clip", id.into())))
    }
}

fn malformed(detail: String) -> CommandError {
    CommandError::Malformed {
        name: "clip".into(),
        detail,
    }
}

fn moved_curve(curve: &EdgeCurve, d: DVec2) -> EdgeCurve {
    match curve {
        EdgeCurve::Line => EdgeCurve::Line,
        EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic {
            c1: *c1 + d,
            c2: *c2 + d,
        },
    }
}

fn moved_segment(s: &PathSegment, d: DVec2) -> PathSegment {
    match s {
        PathSegment::Line { to } => PathSegment::Line { to: *to + d },
        PathSegment::Cubic { c1, c2, to } => PathSegment::Cubic {
            c1: *c1 + d,
            c2: *c2 + d,
            to: *to + d,
        },
    }
}

/// Where pasted pieces go by default: to the right of the project's
/// pieces, level with where they were copied.
fn default_offset(p: &Project, clip: &Clip) -> DVec2 {
    let right = p
        .panels()
        .keys()
        .filter_map(|id| p.panel_bounding_box(*id))
        .map(|b| b.max.x)
        .reduce(f64::max);
    let left = clip.points.values().map(|q| q.pos.x).reduce(f64::min);
    match (right, left) {
        (Some(right), Some(left)) => DVec2::new(right + PASTE_GAP_MM - left, 0.0),
        _ => DVec2::ZERO,
    }
}

impl CommandSpec for PanelPaste {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let clip: Clip =
            serde_json::from_value(self.clip.clone()).map_err(|e| malformed(e.to_string()))?;
        if clip.format != FORMAT {
            return Err(malformed(format!("not a clip (`format` is not {FORMAT})")));
        }
        if clip.version != SCHEMA_VERSION {
            return Err(malformed(format!(
                "copied with another version of the app (schema {}, this one {SCHEMA_VERSION})",
                clip.version
            )));
        }
        if clip.panels.is_empty() {
            return Err(CommandError::invalid("clip", Invalid::Empty));
        }
        let d = match self.offset {
            Some(o) => {
                finite("offset", &o)?;
                v(o)
            }
            None => default_offset(tx.project(), &clip),
        };
        let p = tx.project();
        let garment = match self.garment {
            Some(g) if p.garments().contains_key(&g) => g,
            Some(g) => return Err(CommandError::not_found(g)),
            None => clip
                .panels
                .values()
                .map(|pn| pn.garment)
                .find(|g| p.garments().contains_key(g))
                .or_else(|| p.default_garment())
                .ok_or_else(|| CommandError::Internal {
                    detail: "the project has no garment".into(),
                })?,
        };
        let keep_grade =
            clip.sizes.is_some() && clip.sizes == p.grading().map(|g| (g.sizes.len(), g.base));
        // A piece pasted where another already starts (beside its
        // original) would start inside it: leave it unplaced.
        let taken: Vec<_> = p.panels().values().filter_map(|pn| pn.placement).collect();

        let panels = Fresh(
            clip.panels
                .keys()
                .map(|id| (*id, tx.new_panel_id()))
                .collect(),
        );
        let points = Fresh(
            clip.points
                .keys()
                .map(|id| (*id, tx.new_point_id()))
                .collect(),
        );
        let edges = Fresh(
            clip.edges
                .keys()
                .map(|id| (*id, tx.new_edge_id()))
                .collect(),
        );

        let mut unplaced = Vec::new();
        for (id, panel) in &clip.panels {
            let placement = panel.placement.filter(|place| !taken.contains(place));
            if placement.is_none() {
                unplaced.push(*id);
            }
            let mirror_of = match &panel.mirror_of {
                Some(link) if clip.panels.contains_key(&link.source) => {
                    Some(r3d_core::MirrorLink {
                        source: panels.get(link.source)?,
                        // The axis moves with the pieces; the offset after the
                        // reflection stays.
                        origin: link.origin + d,
                        ..link.clone()
                    })
                }
                _ => None,
            };
            let contour = panel
                .contour
                .iter()
                .map(|e| edges.get(*e))
                .collect::<Result<_, _>>()?;
            let symmetry = match &panel.symmetry {
                Some(sym) => Some(Symmetry {
                    origin: sym.origin + d,
                    dir: sym.dir,
                    start: points.get(sym.start)?,
                    end: points.get(sym.end)?,
                    points: sym
                        .points
                        .iter()
                        .map(|[a, b]| Ok([points.get(*a)?, points.get(*b)?]))
                        .collect::<Result<_, CommandError>>()?,
                    edges: sym
                        .edges
                        .iter()
                        .map(|[a, b]| Ok([edges.get(*a)?, edges.get(*b)?]))
                        .collect::<Result<_, CommandError>>()?,
                }),
                None => None,
            };
            tx.put_panel(
                panels.get(*id)?,
                Panel {
                    name: panel.name.clone(),
                    garment,
                    contour,
                    mirror_of,
                    placement,
                    symmetry,
                    ..panel.clone()
                },
            );
        }
        for (id, point) in &clip.points {
            tx.put_point(
                points.get(*id)?,
                Point {
                    panel: panels.get(point.panel)?,
                    pos: point.pos + d,
                    grade: if keep_grade {
                        point.grade.clone()
                    } else {
                        Vec::new()
                    },
                },
            );
        }
        for (id, edge) in &clip.edges {
            tx.put_edge(
                edges.get(*id)?,
                Edge {
                    panel: panels.get(edge.panel)?,
                    start: points.get(edge.start)?,
                    end: points.get(edge.end)?,
                    curve: moved_curve(&edge.curve, d),
                    grade: if keep_grade {
                        edge.grade.clone()
                    } else {
                        Vec::new()
                    },
                },
            );
        }
        for notch in &clip.notches {
            let id = tx.new_notch_id();
            tx.put_notch(
                id,
                Notch {
                    edge: edges.get(notch.edge)?,
                    ..notch.clone()
                },
            );
        }
        for line in &clip.internal_lines {
            let id = tx.new_internal_line_id();
            tx.put_internal_line(
                id,
                InternalLine {
                    panel: panels.get(line.panel)?,
                    start: line.start + d,
                    segments: line.segments.iter().map(|s| moved_segment(s, d)).collect(),
                },
            );
        }
        for pin in clip
            .pins
            .iter()
            .filter(|pin| !unplaced.contains(&pin.panel))
        {
            let id = tx.new_pin_id();
            tx.put_pin(
                id,
                Pin {
                    panel: panels.get(pin.panel)?,
                    at: pin.at + d,
                    ..*pin
                },
            );
        }
        let mut created: Vec<r3d_core::AnyId> = panels.0.values().map(|id| (*id).into()).collect();
        for seam in &clip.seams {
            let side = |s: SeamSide| -> Result<SeamSide, CommandError> {
                Ok(SeamSide {
                    edge: edges.get(s.edge)?,
                    ..s
                })
            };
            let id = tx.new_seam_id();
            tx.put_seam(
                id,
                Seam {
                    a: side(seam.a)?,
                    b: side(seam.b)?,
                    ..*seam
                },
            );
            created.push(id.into());
        }
        for c in &clip.constraints {
            let c = c
                .remapped(|pt| points.get(pt).ok(), |e| edges.get(e).ok())
                .ok_or_else(|| malformed("a constraint names what is not in the clip".into()))?;
            let id = tx.new_constraint_id();
            tx.put_constraint(id, c);
        }
        // A clip edited by hand may not hold together: say so, rather
        // than an internal error from the session's own check.
        tx.project()
            .check_integrity()
            .map_err(|e| malformed(e.to_string()))?;
        Ok(CommandOutput { created })
    }
}
