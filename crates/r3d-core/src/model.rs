//! The document model.
//!
//! Fields are private: read through accessors, modify through
//! [`crate::edit::Transaction`]. Entity maps are `BTreeMap`s keyed by typed
//! IDs, which gives a stable iteration order and therefore stable JSON.

use std::collections::BTreeMap;

use glam::{DVec2, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::{EdgeId, GarmentId, InternalLineId, NotchId, PanelId, PinId, PointId, SeamId};

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

/// A group of panels worn together. v0.1 uses a single garment, but the
/// model does not assume it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Garment {
    pub name: String,
    /// What the garment is made of.
    pub fabric: Fabric,
    /// Display colour as `#RRGGBB`; `None` uses the fabric's usual colour.
    pub color: Option<String>,
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

/// One side of a seam: a whole edge, walked from its start point unless
/// `reversed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSide {
    pub edge: EdgeId,
    pub reversed: bool,
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

/// Two edges sewn together, point for point along their lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Seam {
    pub a: SeamSide,
    pub b: SeamSide,
    /// The longer side is gathered onto the shorter one on purpose (a
    /// gathered skirt into a waistband, a sleeve cap): their difference
    /// is not a mismatch.
    pub gathered: bool,
}

impl Seam {
    pub fn touches(&self, edge: EdgeId) -> bool {
        self.a.edge == edge || self.b.edge == edge
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
}

/// An edge between two points of the same panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub panel: PanelId,
    pub start: PointId,
    pub end: PointId,
    pub curve: EdgeCurve,
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
}
