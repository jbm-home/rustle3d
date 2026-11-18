//! The document model.
//!
//! Fields are private: read through accessors, modify through
//! [`crate::edit::Transaction`]. Entity maps are `BTreeMap`s keyed by typed
//! IDs, which gives a stable iteration order and therefore stable JSON.

use std::collections::BTreeMap;

use glam::{DVec2, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::{
    ConstraintId, EdgeId, GarmentId, GuideId, InternalLineId, NotchId, PanelId, PinId, PointId,
    SeamId,
};

/// Ordered entity map.
pub type IdMap<K, V> = BTreeMap<K, V>;

/// Unit used to *display* lengths. Storage is always millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayUnit {
    #[default]
    Mm,
    Cm,
    In,
}

/// A Rustle3D project document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub(crate) name: String,
    pub(crate) display_unit: DisplayUnit,
    /// Next raw ID to allocate (shared by all entity kinds).
    pub(crate) next_id: u64,
    pub(crate) garments: IdMap<GarmentId, Garment>,
    pub(crate) panels: IdMap<PanelId, Panel>,
    pub(crate) points: IdMap<PointId, Point>,
    pub(crate) edges: IdMap<EdgeId, Edge>,
    pub(crate) notches: IdMap<NotchId, Notch>,
    pub(crate) internal_lines: IdMap<InternalLineId, InternalLine>,
    /// The body the garment is made for, if any.
    pub(crate) avatar: Option<Avatar>,
    pub(crate) seams: IdMap<SeamId, Seam>,
    pub(crate) pins: IdMap<PinId, Pin>,
    /// The size range the pattern is graded to, if any.
    pub(crate) grading: Option<Grading>,
    /// Constraints and dimensions the drawing keeps.
    pub(crate) constraints: IdMap<ConstraintId, Constraint>,
    /// Construction lines on the board.
    pub(crate) guides: IdMap<GuideId, Guide>,
    /// Named variables for formulas: name → formula (`docs/specs/20-parametric.md`).
    pub(crate) variables: BTreeMap<String, String>,
}

/// A relation the drawing keeps between points and edges: after every
/// change, the constraint solver moves the geometry so that all of them
/// hold again (`docs/specs/19-constraints.md`). Dimensions (`distance`,
/// `length`, `angle`) carry a value; the others are geometric.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Constraint {
    /// The point never moves when the solver adjusts the drawing (it moves
    /// only when it is moved itself).
    Fixed { point: PointId },
    /// Two points at the same place (on two pieces, usually).
    Coincident { a: PointId, b: PointId },
    /// The edge's ends at the same height.
    Horizontal { edge: EdgeId },
    /// The edge's ends one above the other.
    Vertical { edge: EdgeId },
    /// The two edges' chords parallel.
    Parallel { a: EdgeId, b: EdgeId },
    /// The two edges' chords at right angles.
    Perpendicular { a: EdgeId, b: EdgeId },
    /// A smooth joint: the two contour edges meeting at the point leave it
    /// in opposite directions (no corner).
    Tangent { point: PointId },
    /// The two edges have the same length (along their curves).
    EqualLength { a: EdgeId, b: EdgeId },
    /// The point lies on the edge (on its line, extended, for a straight
    /// edge).
    OnEdge { point: PointId, edge: EdgeId },
    /// A dimension: the distance between two points.
    Distance {
        a: PointId,
        b: PointId,
        /// mm, > 0.
        mm: f64,
        #[serde(default)]
        along: Along,
        /// Computes `mm` from the body and the variables (a length).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formula: Option<String>,
    },
    /// A dimension: the edge's length along its curve.
    Length {
        edge: EdgeId,
        /// mm, > 0.
        mm: f64,
        /// Computes `mm` from the body and the variables (a length).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formula: Option<String>,
    },
    /// A dimension: the angle between two edges (at their common point
    /// when they share one, between their chords otherwise).
    Angle {
        a: EdgeId,
        b: EdgeId,
        /// Degrees, 0–180.
        deg: f64,
        /// Computes `deg` from the body and the variables (a plain
        /// number).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formula: Option<String>,
    },
}

/// How a distance is measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Along {
    /// In a straight line.
    #[default]
    Direct,
    /// Across: the difference of the two points' x.
    Horizontal,
    /// Up or down: the difference of their y.
    Vertical,
}

impl Constraint {
    /// The points it names.
    pub fn points(&self) -> Vec<PointId> {
        match self {
            Constraint::Fixed { point }
            | Constraint::Tangent { point }
            | Constraint::OnEdge { point, .. } => vec![*point],
            Constraint::Coincident { a, b } | Constraint::Distance { a, b, .. } => vec![*a, *b],
            _ => Vec::new(),
        }
    }

    /// The edges it names.
    pub fn edges(&self) -> Vec<EdgeId> {
        match self {
            Constraint::Horizontal { edge }
            | Constraint::Vertical { edge }
            | Constraint::Length { edge, .. }
            | Constraint::OnEdge { edge, .. } => vec![*edge],
            Constraint::Parallel { a, b }
            | Constraint::Perpendicular { a, b }
            | Constraint::EqualLength { a, b }
            | Constraint::Angle { a, b, .. } => vec![*a, *b],
            _ => Vec::new(),
        }
    }

    /// A dimension's value (mm or degrees); `None` for the others.
    pub fn value(&self) -> Option<f64> {
        match self {
            Constraint::Distance { mm, .. } | Constraint::Length { mm, .. } => Some(*mm),
            Constraint::Angle { deg, .. } => Some(*deg),
            _ => None,
        }
    }

    /// A dimension's formula, if it has one.
    pub fn formula(&self) -> Option<&str> {
        match self {
            Constraint::Distance { formula, .. }
            | Constraint::Length { formula, .. }
            | Constraint::Angle { formula, .. } => formula.as_deref(),
            _ => None,
        }
    }

    /// The same dimension with another formula (or none); `None` for the
    /// others.
    pub fn with_formula(&self, f: Option<String>) -> Option<Constraint> {
        let mut c = self.clone();
        match &mut c {
            Constraint::Distance { formula, .. }
            | Constraint::Length { formula, .. }
            | Constraint::Angle { formula, .. } => *formula = f,
            _ => return None,
        }
        Some(c)
    }

    /// The length dimension a dimension's value has: 1 (mm) for a
    /// distance or a length, 0 for an angle (degrees).
    pub fn value_power(&self) -> Option<i32> {
        match self {
            Constraint::Distance { .. } | Constraint::Length { .. } => Some(1),
            Constraint::Angle { .. } => Some(0),
            _ => None,
        }
    }

    /// The same dimension with another value; `None` for the others.
    pub fn with_value(&self, value: f64) -> Option<Constraint> {
        let mut c = self.clone();
        match &mut c {
            Constraint::Distance { mm, .. } | Constraint::Length { mm, .. } => *mm = value,
            Constraint::Angle { deg, .. } => *deg = value,
            _ => return None,
        }
        Some(c)
    }

    /// The same constraint on other points and edges (`map_point` and
    /// `map_edge` give each one's replacement); `None` when one has none.
    pub fn remapped(
        &self,
        map_point: impl Fn(PointId) -> Option<PointId>,
        map_edge: impl Fn(EdgeId) -> Option<EdgeId>,
    ) -> Option<Constraint> {
        let mut c = self.clone();
        match &mut c {
            Constraint::Fixed { point } | Constraint::Tangent { point } => {
                *point = map_point(*point)?
            }
            Constraint::Coincident { a, b } | Constraint::Distance { a, b, .. } => {
                *a = map_point(*a)?;
                *b = map_point(*b)?;
            }
            Constraint::Horizontal { edge }
            | Constraint::Vertical { edge }
            | Constraint::Length { edge, .. } => *edge = map_edge(*edge)?,
            Constraint::OnEdge { point, edge } => {
                *point = map_point(*point)?;
                *edge = map_edge(*edge)?;
            }
            Constraint::Parallel { a, b }
            | Constraint::Perpendicular { a, b }
            | Constraint::EqualLength { a, b }
            | Constraint::Angle { a, b, .. } => {
                *a = map_edge(*a)?;
                *b = map_edge(*b)?;
            }
        }
        Some(c)
    }
}

/// A construction line between two board points (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guide {
    pub a: DVec2,
    pub b: DVec2,
}

/// A size range: the pattern is drawn in the base size, and each point's
/// grade moves it to the others (`docs/specs/18-grading.md`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Grading {
    /// From the smallest to the largest; at least two.
    pub sizes: Vec<Size>,
    /// The size the pattern is drawn in (index into `sizes`).
    pub base: usize,
    /// The size draped and shown in 3D (index into `sizes`).
    pub shown: usize,
}

/// One size of a range: its name and how its body differs from the base
/// size's, the project's avatar (cm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Size {
    /// As on the label: "38", "M".
    pub name: String,
    pub height_cm: f64,
    pub bust_cm: f64,
    pub waist_cm: f64,
    pub hips_cm: f64,
}

/// Body measurements (cm), the input of the avatar fitting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// 0 = feminine body, 1 = masculine body.
    pub masculinity: f64,
    pub height_cm: f64,
    pub bust_cm: f64,
    pub waist_cm: f64,
    pub hips_cm: f64,
}

/// The avatar of a project: measurements, and the preset they came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Avatar {
    pub body: Body,
    /// Name of the preset the measurements were taken from, if any.
    pub preset: Option<String>,
}

/// A group of panels worn together, in one fabric and colour. A project
/// holds one or more, worn in layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Garment {
    pub name: String,
    /// What the garment is made of.
    pub fabric: Fabric,
    /// Display colour as `#RRGGBB`; `None` uses the fabric's usual colour.
    pub color: Option<String>,
    /// Where it is worn among the garments: 0 next to the body, higher
    /// over it (a shirt tucked into trousers is below them). Garments
    /// with the same layer are ordered by id.
    pub layer: u32,
    /// The template it was drafted from, so it can be drafted again for
    /// other sizes; `None` for a garment drawn by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateRef>,
}

/// A template and the measurements it was given (cm, by name; the others
/// came from the body).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateRef {
    pub name: String,
    pub sizes: BTreeMap<String, f64>,
}

/// The built-in fabrics (physical and visual presets).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Fabric {
    #[default]
    Cotton,
    Denim,
    Jersey,
    Silk,
    Wool,
}

/// How a fabric looks: shared by the 3D view, the photorealistic renderer
/// and the 3D exports, so they agree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FabricLook {
    /// Usual colour (sRGB, `0xRRGGBB`).
    pub rgb: u32,
    /// Strength of the sheen, the fibres' colour at grazing angles (0 =
    /// matte, 1.6 = silk): strong for silk, a soft halo for wool, faint
    /// for denim.
    pub sheen: f32,
    /// How spread the sheen is (0–1): low for smooth filaments (silk),
    /// high for fuzzy fibres (wool).
    pub sheen_roughness: f32,
    /// Roughness of the surface's specular reflection (0–1).
    pub roughness: f32,
    /// Strength of that specular reflection (0–1): fibres are a poor
    /// mirror, except smooth silk.
    pub specular: f32,
    /// Share of the light a single layer lets through (0–1): thin silk
    /// and cotton glow against the light, denim does not.
    pub translucency: f32,
}

impl Fabric {
    /// The fabric's look.
    pub const fn look(self) -> FabricLook {
        let (rgb, sheen, sheen_roughness, roughness, specular, translucency) = match self {
            Fabric::Cotton => (0xE4DCCD, 0.6, 0.6, 0.85, 0.25, 0.2),
            Fabric::Denim => (0x3E5677, 0.3, 0.7, 0.9, 0.2, 0.03),
            Fabric::Jersey => (0x9A5A6A, 0.5, 0.5, 0.8, 0.25, 0.12),
            Fabric::Silk => (0xC9A46C, 1.6, 0.3, 0.3, 0.9, 0.25),
            Fabric::Wool => (0x5F5B58, 0.9, 0.9, 0.95, 0.1, 0.05),
        };
        FabricLook {
            rgb,
            sheen,
            sheen_roughness,
            roughness,
            specular,
            translucency,
        }
    }
}

impl Garment {
    /// The garment's colour (sRGB, `0xRRGGBB`): its own if it has a valid
    /// one, otherwise its fabric's.
    pub fn rgb(&self) -> u32 {
        self.color
            .as_deref()
            .and_then(|c| c.strip_prefix('#'))
            .filter(|d| d.len() == 6)
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .unwrap_or(self.fabric.look().rgb)
    }
}

/// What a panel is wrapped around when the simulation starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Around {
    /// The torso and legs: a vertical cylinder around the body's axis.
    #[default]
    Body,
    /// The left arm (the body's left, +x): wrapped around the upper arm,
    /// then the forearm past the elbow.
    LeftArm,
    /// The right arm (−x).
    RightArm,
    /// The left leg (+x), like a trouser leg: wrapped around the leg's
    /// axis, and above the crotch around the body's left half.
    LeftLeg,
    /// The right leg (−x).
    RightLeg,
    /// Not wrapped: a flat sheet facing outward at the placement angle,
    /// `offset_mm` from the body's vertical axis (a bag hanging in front
    /// of the body).
    Flat,
}

/// Where a panel sits around the body for the simulation: wrapped on a
/// cylinder around the body's axis or around an arm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub around: Around,
    /// Angle around the body (degrees): 0 = front, 90 = the body's left
    /// side, 180 = back, 270 = right side. Around an arm: 0 = the outside
    /// of the arm, 180 = the side facing the body. Around a leg, as around
    /// the body, with the leg's axis as the centre. The panel's centre goes
    /// there.
    pub angle_deg: f64,
    /// Height of the panel's top above the floor (cm). Around an arm, the
    /// top goes to the arm's axis at that height and the panel runs down
    /// the arm.
    pub top_cm: f64,
    /// Distance between the body and the panel when it starts (mm).
    pub offset_mm: f64,
    /// The pattern abscissa (mm) that goes to `angle_deg`; `None`: the
    /// panel's horizontal centre. The pieces of a cut panel keep the
    /// whole's, so each starts where it was.
    pub centre_x_mm: Option<f64>,
}

/// One side of a seam: a stretch of outline (one or more consecutive
/// contour edges of a piece, in contour order) or an internal line, all of
/// it or part of it, walked from its start unless `reversed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSide {
    /// Consecutive contour edges of one piece, in contour order; empty
    /// when the side is on an internal line.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<EdgeId>,
    /// The internal line the side runs along, instead of edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<InternalLineId>,
    /// Where the side starts along that path (mm from its start); omitted:
    /// at its start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_mm: Option<f64>,
    /// Where it ends (mm from the path's start); omitted: at its end.
    /// Past the path's end, it ends there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_mm: Option<f64>,
    pub reversed: bool,
}

impl SeamSide {
    /// A whole edge, walked from its start unless `reversed`.
    pub fn edge(edge: EdgeId, reversed: bool) -> Self {
        Self {
            edges: vec![edge],
            line: None,
            from_mm: None,
            to_mm: None,
            reversed,
        }
    }

    /// The edge, when the side is exactly one whole edge.
    pub fn whole_edge(&self) -> Option<EdgeId> {
        match (self.edges.as_slice(), self.line, self.from_mm, self.to_mm) {
            ([e], None, None, None) => Some(*e),
            _ => None,
        }
    }

    /// The same side walked the other way.
    pub fn flipped(&self) -> Self {
        Self {
            reversed: !self.reversed,
            ..self.clone()
        }
    }
}

/// A pin: the cloth around a point of a panel is held at a place in 3D.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub panel: PanelId,
    /// The pinned point on the pattern (mm).
    pub at: DVec2,
    /// Where it is held in 3D (m, Y up, the avatar's feet at y = 0).
    pub position: DVec3,
}

/// Two sides sewn together, point for point along their lengths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Seam {
    pub a: SeamSide,
    pub b: SeamSide,
    /// The longer side is gathered onto the shorter one on purpose (a
    /// gathered skirt into a waistband, a sleeve cap): their difference
    /// is not a mismatch.
    pub gathered: bool,
    /// Ease: how much longer side `a` is meant to be than side `b` (mm;
    /// negative, shorter), eased in along the seam (a sleeve cap, a back
    /// shoulder). Only the difference beyond it is a mismatch.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ease_mm: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Seam {
    /// Whether the seam runs along `edge` (all of it or part).
    pub fn touches(&self, edge: EdgeId) -> bool {
        self.a.edges.contains(&edge) || self.b.edges.contains(&edge)
    }

    /// Whether the seam runs along internal line `line`.
    pub fn touches_line(&self, line: InternalLineId) -> bool {
        self.a.line == Some(line) || self.b.line == Some(line)
    }

    /// From whole edge `edge`: its side and the other side, each as an
    /// edge and whether it is walked reversed, when both sides are whole
    /// edges (what the tools reshaping seams work on).
    pub fn across(&self, edge: EdgeId) -> Option<((EdgeId, bool), (EdgeId, bool))> {
        let (a, b) = (self.a.whole_edge()?, self.b.whole_edge()?);
        let (sa, sb) = ((a, self.a.reversed), (b, self.b.reversed));
        if a == edge {
            Some((sa, sb))
        } else if b == edge {
            Some((sb, sa))
        } else {
            None
        }
    }

    /// A seam of two whole edges.
    pub fn whole(a: SeamSide, b: SeamSide) -> Self {
        Self {
            a,
            b,
            gathered: false,
            ease_mm: 0.0,
        }
    }
}

/// A flat pattern panel bounded by a closed loop of edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    pub name: String,
    pub garment: GarmentId,
    /// Closed loop: `edges[i].end == edges[i + 1].start`, last joins first.
    /// Counter-clockwise when created by commands.
    pub contour: Vec<EdgeId>,
    /// Grainline (warp direction) angle in degrees from the board's +X axis.
    pub grain_angle_deg: f64,
    /// Seam allowance added around the panel when exporting (mm, ≥ 0).
    pub seam_allowance_mm: f64,
    /// Set when this panel is a linked mirror copy: its geometry is derived
    /// from `source` and kept in sync by the command layer.
    pub mirror_of: Option<MirrorLink>,
    /// Where the panel starts around the body; unplaced panels are not
    /// simulated.
    pub placement: Option<Placement>,
    /// Set when the panel is symmetric about an axis: half of it is drawn,
    /// the other half follows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symmetry: Option<Symmetry>,
    /// Holes cut in the piece: closed loops of edges inside the contour,
    /// each walked clockwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<EdgeId>>,
}

/// A piece drawn as a half against an axis (a centre front on the fold).
/// The contour runs along the drawn half from `start` to `end`, two points
/// on the axis, then back along the half reflected across the axis. The
/// reflected points and edges are derived: the command layer recomputes
/// them after every change (keeping their IDs, so seams and notches on
/// them stay), and keeps `start` and `end` on the axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symmetry {
    /// A point on the axis (mm).
    pub origin: DVec2,
    /// Direction of the axis (non-zero).
    pub dir: DVec2,
    /// Where the drawn half starts and ends, on the axis.
    pub start: PointId,
    pub end: PointId,
    /// Each point of the drawn half (but its ends) and its reflection.
    pub points: Vec<[PointId; 2]>,
    /// Each edge of the drawn half and its reflection.
    pub edges: Vec<[EdgeId; 2]>,
}

/// Link from a mirror copy to its source panel: the copy is the source
/// reflected across the axis (`origin`, `dir`), then moved by `offset`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorLink {
    pub source: PanelId,
    /// A point on the mirror axis (mm).
    pub origin: DVec2,
    /// Direction of the mirror axis (non-zero).
    pub dir: DVec2,
    /// Translation applied after the reflection (mm), so the copy can be
    /// placed anywhere on the board.
    pub offset: DVec2,
}

/// An alignment mark on an edge, `offset_mm` from the edge's start point
/// along the edge. If the edge becomes shorter, the notch sits at its end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notch {
    pub edge: EdgeId,
    pub offset_mm: f64,
}

/// An open path drawn inside a panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalLine {
    pub panel: PanelId,
    pub start: DVec2,
    /// At least one segment.
    pub segments: Vec<PathSegment>,
}

/// One segment of an open path, starting where the previous one ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathSegment {
    Line { to: DVec2 },
    Cubic { c1: DVec2, c2: DVec2, to: DVec2 },
}

impl PathSegment {
    pub fn end(&self) -> DVec2 {
        match self {
            PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
        }
    }
}

/// A point of a panel contour, in board coordinates (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub panel: PanelId,
    pub pos: DVec2,
    /// Grade rule: where the point is in each size of the project's range,
    /// as an offset from `pos` (mm; zero in the base size). Empty: the
    /// point is the same in every size.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grade: Vec<DVec2>,
}

impl Point {
    /// An ungraded point.
    pub fn new(panel: PanelId, pos: DVec2) -> Self {
        Self {
            panel,
            pos,
            grade: Vec::new(),
        }
    }
}

/// An edge between two points of the same panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub panel: PanelId,
    pub start: PointId,
    pub end: PointId,
    pub curve: EdgeCurve,
    /// Grade of a curve's control points: their offsets in each size of
    /// the project's range (mm; zero in the base size). Empty: they follow
    /// the end points (see `grading`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grade: Vec<[DVec2; 2]>,
}

impl Edge {
    /// An edge whose curve follows its end points from size to size.
    pub fn new(panel: PanelId, start: PointId, end: PointId, curve: EdgeCurve) -> Self {
        Self {
            panel,
            start,
            end,
            curve,
            grade: Vec::new(),
        }
    }
}

/// Shape of an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EdgeCurve {
    /// Straight segment.
    Line,
    /// Cubic Bézier with absolute control points (mm).
    Cubic { c1: DVec2, c2: DVec2 },
}

/// Name given to the default garment of a new project (English default; see
/// the i18n notes in `docs/architecture.md` §7.7).
pub const DEFAULT_GARMENT_NAME: &str = "Garment 1";

impl Project {
    /// A new empty project with one default garment.
    pub fn new(name: impl Into<String>) -> Self {
        let garment = GarmentId::from_raw(1);
        let mut garments = IdMap::new();
        garments.insert(
            garment,
            Garment {
                name: DEFAULT_GARMENT_NAME.to_owned(),
                fabric: Fabric::default(),
                color: None,
                layer: 0,
                template: None,
            },
        );
        Self {
            name: name.into(),
            display_unit: DisplayUnit::default(),
            next_id: 2,
            garments,
            panels: IdMap::new(),
            points: IdMap::new(),
            edges: IdMap::new(),
            notches: IdMap::new(),
            internal_lines: IdMap::new(),
            avatar: None,
            seams: IdMap::new(),
            pins: IdMap::new(),
            grading: None,
            constraints: IdMap::new(),
            guides: IdMap::new(),
            variables: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_unit(&self) -> DisplayUnit {
        self.display_unit
    }

    pub fn garments(&self) -> &IdMap<GarmentId, Garment> {
        &self.garments
    }

    pub fn panels(&self) -> &IdMap<PanelId, Panel> {
        &self.panels
    }

    pub fn points(&self) -> &IdMap<PointId, Point> {
        &self.points
    }

    pub fn edges(&self) -> &IdMap<EdgeId, Edge> {
        &self.edges
    }

    pub fn notches(&self) -> &IdMap<NotchId, Notch> {
        &self.notches
    }

    pub fn seams(&self) -> &IdMap<SeamId, Seam> {
        &self.seams
    }

    pub fn pins(&self) -> &IdMap<PinId, Pin> {
        &self.pins
    }

    /// The seam an edge belongs to, if any.
    pub fn seam_of(&self, edge: EdgeId) -> Option<SeamId> {
        self.seams
            .iter()
            .find(|(_, s)| s.touches(edge))
            .map(|(id, _)| *id)
    }

    pub fn avatar(&self) -> Option<&Avatar> {
        self.avatar.as_ref()
    }

    pub fn grading(&self) -> Option<&Grading> {
        self.grading.as_ref()
    }

    pub fn constraints(&self) -> &IdMap<ConstraintId, Constraint> {
        &self.constraints
    }

    pub fn constraint(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.get(&id)
    }

    pub fn guides(&self) -> &IdMap<GuideId, Guide> {
        &self.guides
    }

    pub fn variables(&self) -> &BTreeMap<String, String> {
        &self.variables
    }

    /// Where formulas are computed in the size drawn: the avatar's body
    /// and the variables.
    pub fn formula_scope(&self) -> crate::formula::Scope<'_> {
        crate::formula::Scope {
            body: self.avatar.as_ref().map(|a| &a.body),
            variables: &self.variables,
        }
    }

    /// The panel a constraint is drawn on: that of its first point, or
    /// else of its first edge.
    pub fn constraint_panel(&self, c: &Constraint) -> Option<PanelId> {
        c.points()
            .first()
            .and_then(|p| self.point(*p).map(|p| p.panel))
            .or_else(|| {
                c.edges()
                    .first()
                    .and_then(|e| self.edge(*e).map(|e| e.panel))
            })
    }

    /// Whether an edge is derived (on a linked mirror copy, or on the
    /// reflected half of a symmetric piece): it follows other geometry, so
    /// it cannot be edited or constrained itself.
    pub fn is_derived_edge(&self, id: EdgeId) -> bool {
        self.edge(id).is_some_and(|e| {
            self.panel(e.panel).is_some_and(|p| {
                p.mirror_of.is_some()
                    || p.symmetry
                        .as_ref()
                        .is_some_and(|s| s.edges.iter().any(|[_, r]| *r == id))
            })
        })
    }

    /// Whether a point is derived: on a linked mirror copy, or on the
    /// reflected half of a symmetric piece (see [`Project::is_derived_edge`]).
    pub fn is_derived_point(&self, id: PointId) -> bool {
        self.point(id)
            .and_then(|p| self.panel(p.panel))
            .is_some_and(|p| {
                p.mirror_of.is_some()
                    || p.symmetry
                        .as_ref()
                        .is_some_and(|s| s.points.iter().any(|[_, r]| *r == id))
            })
    }

    pub fn internal_lines(&self) -> &IdMap<InternalLineId, InternalLine> {
        &self.internal_lines
    }

    pub fn notch(&self, id: NotchId) -> Option<&Notch> {
        self.notches.get(&id)
    }

    pub fn internal_line(&self, id: InternalLineId) -> Option<&InternalLine> {
        self.internal_lines.get(&id)
    }

    pub fn panel(&self, id: PanelId) -> Option<&Panel> {
        self.panels.get(&id)
    }

    pub fn point(&self, id: PointId) -> Option<&Point> {
        self.points.get(&id)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    /// The garment new panels go to: the first one.
    pub fn default_garment(&self) -> Option<GarmentId> {
        self.garments.keys().next().copied()
    }

    /// The garment a panel belongs to.
    pub fn garment_of(&self, panel: PanelId) -> Option<&Garment> {
        self.panel(panel)
            .and_then(|p| self.garments.get(&p.garment))
    }

    /// The garments from the one worn next to the body to the outermost.
    pub fn garments_by_layer(&self) -> Vec<GarmentId> {
        let mut ids: Vec<GarmentId> = self.garments.keys().copied().collect();
        ids.sort_by_key(|id| (self.garments.get(id).map_or(0, |g| g.layer), *id));
        ids
    }
}
