//! The document model.
//!
//! Fields are private: read through accessors, modify through
//! [`crate::edit::Transaction`]. Entity maps are `BTreeMap`s keyed by typed
//! IDs, which gives a stable iteration order and therefore stable JSON.

use std::collections::BTreeMap;

use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::{EdgeId, GarmentId, InternalLineId, NotchId, PanelId, PointId};

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
}

/// A group of panels worn together. v0.1 uses a single garment, but the
/// model does not assume it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Garment {
    pub name: String,
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
