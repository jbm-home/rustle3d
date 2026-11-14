use r3d_core::{AnyId, Edge, EdgeCurve, PanelId, Point, Transaction};
use r3d_geom::DVec2;
use r3d_geom::polygon;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, v};
use crate::error::{CommandError, Invalid, finite, non_blank, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Default grainline: vertical on the board, the usual layout for garments.
pub const DEFAULT_GRAIN_ANGLE_DEG: f64 = 90.0;

fn default_grain() -> f64 {
    DEFAULT_GRAIN_ANGLE_DEG
}

/// Create a panel from a closed straight-edged outline. Shared by the
/// rectangle and polygon commands.
fn create_panel(
    tx: &mut Transaction<'_>,
    name: Option<&str>,
    mut pts: Vec<DVec2>,
    grain_angle_deg: f64,
) -> Result<CommandOutput, CommandError> {
    finite("grain_angle_deg", &[grain_angle_deg])?;
    if let Some(n) = name {
        non_blank("name", n)?;
    }
    if pts.iter().any(|p| !p.is_finite()) {
        return Err(CommandError::invalid("points", Invalid::NotFinite));
    }
    if pts.len() < 3 {
        return Err(CommandError::invalid("points", Invalid::TooFewPoints(3)));
    }
    // A simple polygon (no crossing, overlapping or repeated edges, which
    // also rules out collinear outlines) always has a non-zero area.
    if !polygon::is_simple(&pts) {
        return Err(CommandError::invalid("points", Invalid::NotSimple));
    }
    if polygon::signed_area(&pts) < 0.0 {
        pts.reverse(); // store contours counter-clockwise
    }
    let garment = tx
        .project()
        .default_garment()
        .ok_or_else(|| CommandError::Internal {
            detail: "project has no garment".into(),
        })?;
    // English default name (i18n: see docs/architecture.md §7.7).
    let name = match name {
        Some(n) => n.trim().to_owned(),
        None => format!("Panel {}", tx.project().panels().len() + 1),
    };

    let panel = tx.new_panel_id();
    let point_ids: Vec<_> = pts
        .iter()
        .map(|&pos| {
            let id = tx.new_point_id();
            tx.put_point(id, Point { panel, pos });
            id
        })
        .collect();
    let n = point_ids.len();
    let edge_ids: Vec<_> = (0..n)
        .map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge {
                    panel,
                    start: point_ids[i],
                    end: point_ids[(i + 1) % n],
                    curve: EdgeCurve::Line,
                },
            );
            id
        })
        .collect();
    tx.put_panel(
        panel,
        r3d_core::Panel {
            name,
            garment,
            contour: edge_ids.clone(),
            grain_angle_deg,
        },
    );
    let mut created: Vec<AnyId> = vec![panel.into()];
    created.extend(point_ids.into_iter().map(AnyId::from));
    created.extend(edge_ids.into_iter().map(AnyId::from));
    Ok(CommandOutput { created })
}

/// Create a rectangular panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreateRect {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Bottom-left corner on the board, in mm. Defaults to `[0, 0]`.
    #[serde(default)]
    pub origin: Vec2,
    /// Width in mm (> 0).
    pub width: f64,
    /// Height in mm (> 0).
    pub height: f64,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
}

impl Described for PanelCreateRect {
    const SUMMARY: &'static str = "Create a rectangular panel. Returns the panel, then its 4 points (counter-clockwise from the origin), then its 4 edges.";
}

impl CommandSpec for PanelCreateRect {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("origin", &self.origin)?;
        positive("width", self.width)?;
        positive("height", self.height)?;
        let o = v(self.origin);
        let pts = vec![
            o,
            o + DVec2::new(self.width, 0.0),
            o + DVec2::new(self.width, self.height),
            o + DVec2::new(0.0, self.height),
        ];
        create_panel(tx, self.name.as_deref(), pts, self.grain_angle_deg)
    }
}

/// Create a panel from a closed polygon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreatePolygon {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// At least 3 vertices in mm; the closing edge is implicit. Must form a
    /// simple outline (no crossing or overlapping edges). Stored
    /// counter-clockwise.
    pub points: Vec<Vec2>,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
}

impl Described for PanelCreatePolygon {
    const SUMMARY: &'static str =
        "Create a panel from a closed polygon. Returns the panel, then its points, then its edges.";
}

impl CommandSpec for PanelCreatePolygon {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let pts = self.points.iter().map(|p| v(*p)).collect();
        create_panel(tx, self.name.as_deref(), pts, self.grain_angle_deg)
    }
}

fn get_panel(tx: &Transaction<'_>, id: PanelId) -> Result<r3d_core::Panel, CommandError> {
    tx.project()
        .panel(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

/// Rename a panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelRename {
    pub panel: PanelId,
    /// New name.
    pub name: String,
}

impl Described for PanelRename {
    const SUMMARY: &'static str = "Rename a panel.";
}

impl CommandSpec for PanelRename {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_blank("name", &self.name)?;
        let mut p = get_panel(tx, self.panel)?;
        p.name = self.name.trim().to_owned();
        tx.put_panel(self.panel, p);
        Ok(CommandOutput::default())
    }
}

/// Move a whole panel on the board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelTranslate {
    pub panel: PanelId,
    /// Displacement in mm.
    pub delta: Vec2,
}

impl Described for PanelTranslate {
    const SUMMARY: &'static str = "Move a whole panel on the board by a displacement in mm.";
}

impl CommandSpec for PanelTranslate {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("delta", &self.delta)?;
        let d = v(self.delta);
        let panel = get_panel(tx, self.panel)?;
        for eid in &panel.contour {
            let edge = tx
                .project()
                .edge(*eid)
                .cloned()
                .ok_or_else(|| CommandError::not_found(*eid))?;
            let point = tx
                .project()
                .point(edge.start)
                .cloned()
                .ok_or_else(|| CommandError::not_found(edge.start))?;
            tx.put_point(
                edge.start,
                Point {
                    pos: point.pos + d,
                    ..point
                },
            );
            if let EdgeCurve::Cubic { c1, c2 } = edge.curve {
                tx.put_edge(
                    *eid,
                    Edge {
                        curve: EdgeCurve::Cubic {
                            c1: c1 + d,
                            c2: c2 + d,
                        },
                        ..edge
                    },
                );
            }
        }
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.translate:{}", self.panel))
    }
}

/// Delete a panel with its points and edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelDelete {
    pub panel: PanelId,
    /// Must be `true`: deleting is destructive (still undoable).
    #[serde(default)]
    pub confirm: bool,
}

impl Described for PanelDelete {
    const SUMMARY: &'static str =
        "Delete a panel with its points and edges. Destructive: requires \"confirm\": true.";
}

impl CommandSpec for PanelDelete {
    const DESTRUCTIVE: bool = true;

    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let panel = get_panel(tx, self.panel)?;
        for eid in &panel.contour {
            if let Some(edge) = tx.remove_edge(*eid) {
                tx.remove_point(edge.start);
            }
        }
        tx.remove_panel(self.panel);
        Ok(CommandOutput::default())
    }

    fn confirmed(&self) -> bool {
        self.confirm
    }
}
