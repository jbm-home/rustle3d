//! Turning and scaling a whole panel on the board.
//!
//! Both map every position the panel owns through one affine map of the
//! board ([`map_panel`]): its points and curve handles (outline and
//! holes), their grades, its internal lines, pins and buttons, its
//! grainline, print, symmetry axis and placement abscissa. Its dimensions
//! and constraints follow when they can (a horizontal edge turned a
//! quarter becomes vertical; a distance scaled with the piece); a change
//! one of them could not follow is refused rather than undone by the
//! solver.

use glam::{DAffine2, DMat2};
use r3d_core::{
    Along, Constraint, ConstraintId, Edge, EdgeCurve, Panel, PanelId, Pin, Point, Transaction,
};
use r3d_geom::{DVec2, polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, editable_panel, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Turn a whole panel on the board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelRotate {
    pub panel: PanelId,
    /// Degrees, counter-clockwise on the board (+X towards +Y).
    pub angle_deg: f64,
    /// The point turned about (mm). Defaults to the centre of the panel's
    /// bounding box.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre: Option<Vec2>,
}

impl Described for PanelRotate {
    const SUMMARY: &'static str = "Turn a whole panel on the board by an angle (degrees, counter-clockwise), about a point (default: the centre of its bounding box). Its lines, pins, buttons, grainline and print turn with it; horizontal and vertical constraints swap on a quarter turn, and any other angle is refused while the panel has them. Not on a linked mirror copy (turn its source).";
}

impl CommandSpec for PanelRotate {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("angle_deg", &[self.angle_deg])?;
        let panel = editable_panel(tx, self.panel)?;
        let centre = centre_of(tx, self.panel, self.centre)?;
        // A whole number of quarter turns keeps the board's axes.
        let quarters = self.angle_deg / 90.0;
        let quarters = ((quarters - quarters.round()).abs() < 1e-9)
            .then(|| (quarters.round() as i64).rem_euclid(4));
        let mut updates = Vec::new();
        for (id, c) in own_constraints(tx, self.panel) {
            let turned = match (&c, quarters) {
                (_, Some(0 | 2)) => continue,
                (Constraint::Horizontal { edge }, Some(_)) => Constraint::Vertical { edge: *edge },
                (Constraint::Vertical { edge }, Some(_)) => Constraint::Horizontal { edge: *edge },
                (
                    Constraint::Distance {
                        along: Along::Horizontal,
                        ..
                    },
                    Some(_),
                ) => with_along(&c, Along::Vertical),
                (
                    Constraint::Distance {
                        along: Along::Vertical,
                        ..
                    },
                    Some(_),
                ) => with_along(&c, Along::Horizontal),
                (
                    Constraint::Horizontal { .. }
                    | Constraint::Vertical { .. }
                    | Constraint::Distance {
                        along: Along::Horizontal | Along::Vertical,
                        ..
                    },
                    None,
                ) => return Err(CommandError::invalid("angle_deg", Invalid::Constrained)),
                _ => continue,
            };
            updates.push((id, turned));
        }
        for (id, c) in updates {
            tx.put_constraint(id, c);
        }
        let map = DAffine2::from_translation(centre)
            * DAffine2::from_angle(self.angle_deg.to_radians())
            * DAffine2::from_translation(-centre);
        map_panel(tx, self.panel, panel, map)?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.rotate:{}", self.panel))
    }
}

/// Scale a whole panel on the board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelScale {
    pub panel: PanelId,
    /// Factors across (x) and up and down (y), > 0; `[1.1, 1.1]` makes the
    /// panel 10 % bigger every way.
    pub factor: Vec2,
    /// The point that stays put (mm). Defaults to the centre of the panel's
    /// bounding box.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre: Option<Vec2>,
}

impl Described for PanelScale {
    const SUMMARY: &'static str = "Scale a whole panel on the board by a factor across and one up and down (> 0), about a point (default: the centre of its bounding box). Its lines, pins and buttons follow; its dimensions are scaled with it. Refused while a dimension has a formula, or when the two factors differ and a length, a straight distance, an angle or a right angle would no longer hold. Not on a linked mirror copy (scale its source).";
}

impl CommandSpec for PanelScale {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("factor", &self.factor)?;
        let [sx, sy] = self.factor;
        if sx <= 0.0 || sy <= 0.0 {
            return Err(CommandError::invalid("factor", Invalid::NotPositive));
        }
        let panel = editable_panel(tx, self.panel)?;
        let centre = centre_of(tx, self.panel, self.centre)?;
        let uniform = (sx - sy).abs() <= 1e-12 * sx.max(sy);
        let mut updates = Vec::new();
        for (id, c) in own_constraints(tx, self.panel) {
            if c.formula().is_some() {
                return Err(CommandError::invalid("factor", Invalid::Constrained));
            }
            let factor = match &c {
                Constraint::Distance {
                    along: Along::Horizontal,
                    ..
                } => sx,
                Constraint::Distance {
                    along: Along::Vertical,
                    ..
                } => sy,
                Constraint::Distance { .. } | Constraint::Length { .. } if uniform => sx,
                Constraint::Angle { .. } | Constraint::Perpendicular { .. } if uniform => continue,
                Constraint::Distance { .. }
                | Constraint::Length { .. }
                | Constraint::Angle { .. }
                | Constraint::Perpendicular { .. } => {
                    return Err(CommandError::invalid("factor", Invalid::Constrained));
                }
                _ => continue,
            };
            if let Some(scaled) = c.value().and_then(|mm| c.with_value(mm * factor)) {
                updates.push((id, scaled));
            }
        }
        for (id, c) in updates {
            tx.put_constraint(id, c);
        }
        let map = DAffine2::from_translation(centre)
            * DAffine2::from_scale(DVec2::new(sx, sy))
            * DAffine2::from_translation(-centre);
        map_panel(tx, self.panel, panel, map)?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.scale:{}", self.panel))
    }
}

/// `centre`, or the centre of the panel's bounding box.
fn centre_of(
    tx: &Transaction<'_>,
    panel: PanelId,
    centre: Option<Vec2>,
) -> Result<DVec2, CommandError> {
    if let Some(c) = centre {
        finite("centre", &c)?;
        return Ok(v(c));
    }
    tx.project()
        .panel_outline(panel)
        .as_deref()
        .and_then(polygon::bounding_box)
        .map(|b| (b.min + b.max) / 2.0)
        .ok_or_else(|| CommandError::not_found(panel))
}

/// The constraints that name only points and edges of `panel`.
fn own_constraints(tx: &Transaction<'_>, panel: PanelId) -> Vec<(ConstraintId, Constraint)> {
    let p = tx.project();
    p.constraints()
        .iter()
        .filter(|(_, c)| {
            c.points()
                .iter()
                .all(|id| p.point(*id).is_some_and(|pt| pt.panel == panel))
                && c.edges()
                    .iter()
                    .all(|id| p.edge(*id).is_some_and(|e| e.panel == panel))
        })
        .map(|(id, c)| (*id, c.clone()))
        .collect()
}

fn with_along(c: &Constraint, to: Along) -> Constraint {
    match c.clone() {
        Constraint::Distance {
            a, b, mm, formula, ..
        } => Constraint::Distance {
            a,
            b,
            mm,
            along: to,
            formula,
        },
        other => other,
    }
}

/// Map every position `panel` owns through `map` (see the module doc).
/// `before` is the panel as it is now.
pub(crate) fn map_panel(
    tx: &mut Transaction<'_>,
    id: PanelId,
    before: Panel,
    map: DAffine2,
) -> Result<(), CommandError> {
    let at = |p: DVec2| map.transform_point2(p);
    let linear: DMat2 = map.matrix2;
    let by = |d: DVec2| linear * d;
    // A direction given as an angle (degrees from +X), mapped (as it is
    // when the map only moves things).
    let turned = |deg: f64| {
        if linear == DMat2::IDENTITY {
            return deg;
        }
        let d = by(DVec2::from_angle(deg.to_radians()));
        // In [0, 360): a hair below 0 is 0, not 360.
        let turned = d.y.atan2(d.x).to_degrees();
        if turned.abs() < 1e-9 {
            0.0
        } else {
            turned.rem_euclid(360.0)
        }
    };
    let p = tx.project();
    // The panel's middle height, before it moves.
    let middle_y = p
        .panel_outline(id)
        .as_deref()
        .and_then(polygon::bounding_box)
        .map_or(0.0, |b| (b.min.y + b.max.y) / 2.0);
    let points: Vec<(_, Point)> = p
        .points()
        .iter()
        .filter(|(_, pt)| pt.panel == id)
        .map(|(i, pt)| (*i, pt.clone()))
        .collect();
    let edges: Vec<(_, Edge)> = p
        .edges()
        .iter()
        .filter(|(_, e)| e.panel == id)
        .map(|(i, e)| (*i, e.clone()))
        .collect();
    let lines: Vec<_> = p
        .internal_lines()
        .iter()
        .filter(|(_, l)| l.panel == id)
        .map(|(i, l)| (*i, super::internal_line::mapped(l, at)))
        .collect();
    let pins: Vec<_> = p
        .pins()
        .iter()
        .filter(|(_, pin)| pin.panel == id)
        .map(|(i, pin)| {
            (
                *i,
                Pin {
                    at: at(pin.at),
                    ..*pin
                },
            )
        })
        .collect();
    let buttons: Vec<_> = p
        .buttons()
        .iter()
        .filter(|(_, b)| b.panel == id || b.hole.panel == id)
        .map(|(i, b)| (*i, b.clone()))
        .collect();
    for (i, pt) in points {
        tx.put_point(
            i,
            Point {
                pos: at(pt.pos),
                grade: pt.grade.iter().map(|g| by(*g)).collect(),
                ..pt
            },
        );
    }
    for (i, e) in edges {
        let curve = match e.curve {
            EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic {
                c1: at(c1),
                c2: at(c2),
            },
            EdgeCurve::Line => EdgeCurve::Line,
        };
        let grade = e.grade.iter().map(|[a, b]| [by(*a), by(*b)]).collect();
        tx.put_edge(i, Edge { curve, grade, ..e });
    }
    for (i, line) in lines {
        tx.put_internal_line(i, line);
    }
    for (i, pin) in pins {
        tx.put_pin(i, pin);
    }
    for (i, mut b) in buttons {
        if b.panel == id {
            b.at = at(b.at);
        }
        if b.hole.panel == id {
            b.hole.at = at(b.hole.at);
            b.hole.angle_deg = turned(b.hole.angle_deg);
        }
        tx.put_button(i, b);
    }
    let mut after = before.clone();
    after.grain_angle_deg = turned(before.grain_angle_deg);
    after.print.offset_mm = by(before.print.offset_mm);
    after.print.angle_deg = turned(before.print.angle_deg);
    if let Some(sym) = &mut after.symmetry {
        sym.origin = at(sym.origin);
        sym.dir = by(sym.dir);
    }
    // The pattern abscissa the placement is centred on, at the panel's
    // middle height.
    if let Some(place) = &mut after.placement
        && let Some(x) = place.centre_x_mm
    {
        place.centre_x_mm = Some(at(DVec2::new(x, middle_y)).x);
    }
    if after != before {
        tx.put_panel(id, after);
    }
    Ok(())
}
