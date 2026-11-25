//! Semantic copy and paste of pieces: `panel.copy` gives a clip of pieces
//! with what belongs to them (their points, edges, notches, internal
//! lines, pins, grade rules, construction details, buttons and zippers,
//! the seams between them, their constraints and the construction lines
//! these use, and the mirror links among them); `panel.paste` adds a
//! clip to a project, in the same project or another one, under fresh
//! IDs. `garment.paste` carries whole garments the same way.

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::file::SCHEMA_VERSION;
use r3d_core::{
    AnyId, Button, Constraint, Detail, Edge, EdgeCurve, EdgeId, GarmentId, Guide, GuideId,
    InternalLine, InternalLineId, Notch, NotchId, Panel, PanelId, PathSegment, Pin, Point, PointId,
    Project, Remap, Seam, Symmetry, Transaction, Zipper,
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
pub(crate) struct Clip {
    format: String,
    version: u32,
    /// The size range's number of sizes and base size, if graded: grade
    /// rules paste only into the same layout.
    sizes: Option<(usize, usize)>,
    panels: BTreeMap<PanelId, Panel>,
    points: BTreeMap<PointId, Point>,
    edges: BTreeMap<EdgeId, Edge>,
    notches: BTreeMap<NotchId, Notch>,
    internal_lines: BTreeMap<InternalLineId, InternalLine>,
    seams: Vec<Seam>,
    pins: Vec<Pin>,
    /// Constraints whose points, edges, notches and lines are all in the
    /// clip.
    constraints: Vec<Constraint>,
    /// The construction lines those constraints use.
    guides: BTreeMap<GuideId, Guide>,
    details: Vec<Detail>,
    buttons: Vec<Button>,
    zippers: Vec<Zipper>,
}

/// The IDs a clip holds, each kept as it is.
struct Within<'a>(&'a Clip);

impl Remap for Within<'_> {
    fn panel(&self, id: PanelId) -> Option<PanelId> {
        self.0.panels.contains_key(&id).then_some(id)
    }
    fn point(&self, id: PointId) -> Option<PointId> {
        self.0.points.contains_key(&id).then_some(id)
    }
    fn edge(&self, id: EdgeId) -> Option<EdgeId> {
        self.0.edges.contains_key(&id).then_some(id)
    }
    fn line(&self, id: InternalLineId) -> Option<InternalLineId> {
        self.0.internal_lines.contains_key(&id).then_some(id)
    }
    fn notch(&self, id: NotchId) -> Option<NotchId> {
        self.0.notches.contains_key(&id).then_some(id)
    }
    /// Construction lines are copied with the constraints that use them.
    fn guide(&self, id: GuideId) -> Option<GuideId> {
        Some(id)
    }
}

/// A clip of `panels` of `p`, each with what belongs to it.
pub(crate) fn copy(p: &Project, panels: &[PanelId]) -> Result<Clip, CommandError> {
    if panels.is_empty() {
        return Err(CommandError::invalid("panels", Invalid::Empty));
    }
    let mut clip = Clip {
        format: FORMAT.into(),
        version: SCHEMA_VERSION,
        sizes: p.grading().map(|g| (g.sizes.len(), g.base)),
        panels: BTreeMap::new(),
        points: BTreeMap::new(),
        edges: BTreeMap::new(),
        notches: BTreeMap::new(),
        internal_lines: BTreeMap::new(),
        seams: Vec::new(),
        pins: Vec::new(),
        constraints: Vec::new(),
        guides: BTreeMap::new(),
        details: Vec::new(),
        buttons: Vec::new(),
        zippers: Vec::new(),
    };
    for id in panels {
        let panel = p.panel(*id).ok_or_else(|| CommandError::not_found(*id))?;
        clip.panels.insert(*id, panel.clone());
    }
    let ours = |panel: &PanelId| clip.panels.contains_key(panel);
    let points = owned(p.points(), |pt| ours(&pt.panel));
    let edges = owned(p.edges(), |e| ours(&e.panel));
    let internal_lines = owned(p.internal_lines(), |l| ours(&l.panel));
    let pins: Vec<Pin> = p
        .pins()
        .values()
        .filter(|pin| ours(&pin.panel))
        .copied()
        .collect();
    clip.points = points;
    clip.edges = edges;
    clip.internal_lines = internal_lines;
    clip.pins = pins;
    clip.notches = owned(p.notches(), |n| clip.edges.contains_key(&n.edge));
    let within = Within(&clip);
    let seams = p
        .seams()
        .values()
        .filter(|s| s.a.remapped(&within).is_some() && s.b.remapped(&within).is_some())
        .cloned()
        .collect();
    let constraints: Vec<Constraint> = p
        .constraints()
        .values()
        .filter(|c| c.remapped(&within).is_some())
        .cloned()
        .collect();
    let details = p
        .details()
        .values()
        .filter_map(|d| d.remapped(&within))
        .collect();
    let buttons = p
        .buttons()
        .values()
        .filter_map(|b| b.remapped(&within))
        .collect();
    let zippers = p
        .zippers()
        .values()
        .filter_map(|z| z.remapped(&within))
        .collect();
    clip.guides = constraints
        .iter()
        .filter_map(|c| match c {
            Constraint::OnGuide { guide, .. } | Constraint::GuideDistance { guide, .. } => {
                Some(*guide)
            }
            _ => None,
        })
        .filter_map(|g| Some((g, p.guides().get(&g)?.clone())))
        .collect();
    clip.constraints = constraints
        .into_iter()
        .filter(|c| match c {
            Constraint::OnGuide { guide, .. } | Constraint::GuideDistance { guide, .. } => {
                clip.guides.contains_key(guide)
            }
            _ => true,
        })
        .collect();
    clip.seams = seams;
    clip.details = details;
    clip.buttons = buttons;
    clip.zippers = zippers;
    Ok(clip)
}

/// The entries of `map` that `keep` keeps.
fn owned<K: Ord + Copy, V: Clone>(
    map: &BTreeMap<K, V>,
    keep: impl Fn(&V) -> bool,
) -> BTreeMap<K, V> {
    map.iter()
        .filter(|(_, v)| keep(v))
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}

/// Copy pieces: a clip to give to panel.paste.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCopy {
    /// The pieces to copy (at least one).
    pub panels: Vec<PanelId>,
}

impl Described for PanelCopy {
    const SUMMARY: &'static str = "Copy pieces into a clip (JSON) for panel.paste, in this project or another one: the pieces with their points, curves, notches, internal lines, pins, grade rules, construction details (elastics, topstitching, interfacing, quilting), buttons and zippers, the seams between them, their constraints with the construction lines these use, and the mirror links among them. Seams, trims and links to pieces left out are not copied.";
}

impl QuerySpec for PanelCopy {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        to_value(&copy(p, &self.panels)?)
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
    const SUMMARY: &'static str = "Paste pieces copied with panel.copy, under new IDs, with the seams between them, their notches, internal lines, construction details, buttons, zippers, constraints (a dimension keeps its name unless the project has one by that name), grade rules (when the project has the same number of sizes and base size), pins and mirror links. A piece keeps its place around the body unless a piece of the project already starts there (pasted beside its original): then it is left unplaced, with no pins. Returns the new panels, then the new seams.";
}

/// IDs of the clip mapped to fresh ones.
struct Fresh<K>(BTreeMap<K, K>);

impl<K: Ord + Copy> Fresh<K> {
    fn of(keys: impl Iterator<Item = K>, mut new: impl FnMut() -> K) -> Self {
        Fresh(keys.map(|k| (k, new())).collect())
    }

    fn get(&self, id: K) -> Option<K> {
        self.0.get(&id).copied()
    }
}

/// Every ID of a clip and its fresh one.
struct FreshIds {
    panels: Fresh<PanelId>,
    points: Fresh<PointId>,
    edges: Fresh<EdgeId>,
    lines: Fresh<InternalLineId>,
    notches: Fresh<NotchId>,
    guides: Fresh<GuideId>,
}

impl Remap for FreshIds {
    fn panel(&self, id: PanelId) -> Option<PanelId> {
        self.panels.get(id)
    }
    fn point(&self, id: PointId) -> Option<PointId> {
        self.points.get(id)
    }
    fn edge(&self, id: EdgeId) -> Option<EdgeId> {
        self.edges.get(id)
    }
    fn line(&self, id: InternalLineId) -> Option<InternalLineId> {
        self.lines.get(id)
    }
    fn notch(&self, id: NotchId) -> Option<NotchId> {
        self.notches.get(id)
    }
    fn guide(&self, id: GuideId) -> Option<GuideId> {
        self.guides.get(id)
    }
}

fn malformed(detail: String) -> CommandError {
    CommandError::Malformed {
        name: "clip".into(),
        detail,
    }
}

/// An entity names what the clip does not hold.
fn missing(what: &str) -> CommandError {
    malformed(format!("{what} names what is not in the clip"))
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
pub(crate) fn default_offset(p: &Project, clip: &Clip) -> DVec2 {
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

/// How a clip is pasted.
pub(crate) struct Paste {
    /// How far the pieces move on the board (mm).
    pub offset: DVec2,
    /// The garment they join.
    pub garment: GarmentId,
    /// Keep every piece's place around the body, even where a piece of
    /// the project already starts (a garment worn over another).
    pub keep_places: bool,
}

/// A pasted clip: what was created (the panels, then the seams) and each
/// piece's new ID.
pub(crate) struct Pasted {
    pub created: Vec<AnyId>,
    pub panels: BTreeMap<PanelId, PanelId>,
}

/// Paste `clip` into the project under fresh IDs.
pub(crate) fn paste(
    tx: &mut Transaction<'_>,
    clip: &Clip,
    how: &Paste,
) -> Result<Pasted, CommandError> {
    if clip.panels.is_empty() {
        return Err(CommandError::invalid("clip", Invalid::Empty));
    }
    let d = how.offset;
    let p = tx.project();
    let keep_grade =
        clip.sizes.is_some() && clip.sizes == p.grading().map(|g| (g.sizes.len(), g.base));
    // A piece pasted where another already starts (beside its original)
    // would start inside it: leave it unplaced.
    let taken: Vec<_> = if how.keep_places {
        Vec::new()
    } else {
        p.panels().values().filter_map(|pn| pn.placement).collect()
    };
    let mut names: BTreeSet<String> = p
        .constraints()
        .values()
        .filter_map(|c| c.name().map(str::to_owned))
        .collect();

    let ids = FreshIds {
        panels: Fresh::of(clip.panels.keys().copied(), || tx.new_panel_id()),
        points: Fresh::of(clip.points.keys().copied(), || tx.new_point_id()),
        edges: Fresh::of(clip.edges.keys().copied(), || tx.new_edge_id()),
        lines: Fresh::of(clip.internal_lines.keys().copied(), || {
            tx.new_internal_line_id()
        }),
        notches: Fresh::of(clip.notches.keys().copied(), || tx.new_notch_id()),
        guides: Fresh::of(clip.guides.keys().copied(), || tx.new_guide_id()),
    };
    let panel = |id| ids.panel(id).ok_or_else(|| missing("a piece"));
    let point = |id| ids.point(id).ok_or_else(|| missing("a point"));
    let edge = |id| ids.edge(id).ok_or_else(|| missing("an edge"));

    let mut unplaced = Vec::new();
    for (id, pn) in &clip.panels {
        let placement = pn.placement.filter(|place| !taken.contains(place));
        if placement.is_none() {
            unplaced.push(*id);
        }
        let mirror_of = match &pn.mirror_of {
            Some(link) if clip.panels.contains_key(&link.source) => Some(r3d_core::MirrorLink {
                source: panel(link.source)?,
                // The axis moves with the pieces; the offset after the
                // reflection stays.
                origin: link.origin + d,
                ..link.clone()
            }),
            _ => None,
        };
        let contour = pn
            .contour
            .iter()
            .map(|e| edge(*e))
            .collect::<Result<_, _>>()?;
        let holes = pn
            .holes
            .iter()
            .map(|h| h.iter().map(|e| edge(*e)).collect())
            .collect::<Result<_, _>>()?;
        let symmetry = match &pn.symmetry {
            Some(sym) => Some(Symmetry {
                origin: sym.origin + d,
                dir: sym.dir,
                start: point(sym.start)?,
                end: point(sym.end)?,
                points: sym
                    .points
                    .iter()
                    .map(|[a, b]| Ok([point(*a)?, point(*b)?]))
                    .collect::<Result<_, CommandError>>()?,
                edges: sym
                    .edges
                    .iter()
                    .map(|[a, b]| Ok([edge(*a)?, edge(*b)?]))
                    .collect::<Result<_, CommandError>>()?,
            }),
            None => None,
        };
        tx.put_panel(
            panel(*id)?,
            Panel {
                name: pn.name.clone(),
                garment: how.garment,
                contour,
                mirror_of,
                placement,
                symmetry,
                holes,
                ..pn.clone()
            },
        );
    }
    for (id, pt) in &clip.points {
        tx.put_point(
            point(*id)?,
            Point {
                panel: panel(pt.panel)?,
                pos: pt.pos + d,
                grade: if keep_grade {
                    pt.grade.clone()
                } else {
                    Vec::new()
                },
            },
        );
    }
    for (id, e) in &clip.edges {
        tx.put_edge(
            edge(*id)?,
            Edge {
                panel: panel(e.panel)?,
                start: point(e.start)?,
                end: point(e.end)?,
                curve: moved_curve(&e.curve, d),
                grade: if keep_grade {
                    e.grade.clone()
                } else {
                    Vec::new()
                },
            },
        );
    }
    for (id, notch) in &clip.notches {
        tx.put_notch(
            ids.notch(*id).ok_or_else(|| missing("a notch"))?,
            Notch {
                edge: edge(notch.edge)?,
                ..notch.clone()
            },
        );
    }
    for (id, line) in &clip.internal_lines {
        tx.put_internal_line(
            ids.line(*id).ok_or_else(|| missing("an internal line"))?,
            InternalLine {
                panel: panel(line.panel)?,
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
                panel: panel(pin.panel)?,
                at: pin.at + d,
                ..*pin
            },
        );
    }
    let mut created: Vec<AnyId> = ids.panels.0.values().map(|id| (*id).into()).collect();
    for seam in &clip.seams {
        let id = tx.new_seam_id();
        tx.put_seam(
            id,
            Seam {
                a: seam.a.remapped(&ids).ok_or_else(|| missing("a seam"))?,
                b: seam.b.remapped(&ids).ok_or_else(|| missing("a seam"))?,
                ..seam.clone()
            },
        );
        created.push(id.into());
    }
    for (id, g) in &clip.guides {
        tx.put_guide(
            ids.guide(*id)
                .ok_or_else(|| missing("a construction line"))?,
            Guide {
                a: g.a + d,
                b: g.b + d,
            },
        );
    }
    for c in &clip.constraints {
        let c = c.remapped(&ids).ok_or_else(|| missing("a constraint"))?;
        // Dimension names are unique: one the project has already is
        // dropped.
        let taken = c.name().is_some_and(|name| !names.insert(name.to_owned()));
        let c = if taken {
            c.with_name(None).unwrap_or(c)
        } else {
            c
        };
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
    }
    for detail in &clip.details {
        let id = tx.new_detail_id();
        tx.put_detail(
            id,
            detail
                .remapped(&ids)
                .ok_or_else(|| missing("a construction detail"))?,
        );
    }
    for button in &clip.buttons {
        let b = button.remapped(&ids).ok_or_else(|| missing("a button"))?;
        let id = tx.new_button_id();
        tx.put_button(
            id,
            Button {
                at: b.at + d,
                hole: r3d_core::Buttonhole {
                    at: b.hole.at + d,
                    ..b.hole.clone()
                },
                ..b
            },
        );
    }
    for zipper in &clip.zippers {
        let id = tx.new_zipper_id();
        tx.put_zipper(
            id,
            zipper.remapped(&ids).ok_or_else(|| missing("a zipper"))?,
        );
    }
    Ok(Pasted {
        created,
        panels: ids.panels.0,
    })
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
        let offset = match self.offset {
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
        let pasted = paste(
            tx,
            &clip,
            &Paste {
                offset,
                garment,
                keep_places: false,
            },
        )?;
        // A clip edited by hand may not hold together: say so, rather
        // than an internal error from the session's own check.
        tx.project()
            .check_integrity()
            .map_err(|e| malformed(e.to_string()))?;
        Ok(CommandOutput {
            created: pasted.created,
        })
    }
}
