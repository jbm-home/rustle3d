//! Interfacing (`docs/specs/31-construction-details-2.md`).

use r3d_core::detail::INTERFACING_BAND_MM;
use r3d_core::{
    Detail, DetailId, EdgeId, Interfacing, InterfacingArea, InterfacingFabric, InterfacingWeight,
    InternalLineId, PanelId, Project, Transaction,
};
use r3d_geom::DVec2;
use r3d_geom::polygon::{contains, segments_intersect};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::get_panel;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Where an interfacing goes on its piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AreaSpec {
    /// The whole piece.
    Whole,
    /// A band `width_mm` wide inside consecutive outline edges, in their
    /// order around the piece.
    Band { edges: Vec<EdgeId>, width_mm: f64 },
    /// Inside a closed internal line of the piece.
    Inside { line: InternalLineId },
}

pub(crate) fn get_interfacing(p: &Project, id: DetailId) -> Result<Interfacing, CommandError> {
    match p.details().get(&id) {
        Some(Detail::Interfacing(i)) => Ok(i.clone()),
        Some(_) => Err(CommandError::invalid("interfacing", Invalid::OtherKind)),
        None => Err(CommandError::not_found(id)),
    }
}

/// Two closed outlines share some area: one's point inside the other, or
/// their edges cross.
pub(crate) fn overlap(a: &[DVec2], b: &[DVec2]) -> bool {
    let edges = |l: &[DVec2]| -> Vec<(DVec2, DVec2)> {
        (0..l.len())
            .filter_map(|i| Some((*l.get(i)?, *l.get((i + 1) % l.len())?)))
            .collect()
    };
    let inner = |l: &[DVec2], other: &[DVec2]| {
        // Points a hair inside `l`, so shared edges do not count.
        let c = l.iter().copied().sum::<DVec2>() / l.len().max(1) as f64;
        l.iter().any(|p| contains(other, p.lerp(c, 0.01)))
    };
    inner(a, b)
        || inner(b, a)
        || edges(a).iter().any(|(p, q)| {
            edges(b).iter().any(|(r, s)| {
                // Crossing, not merely touching at a shared corner.
                segments_intersect(*p, *q, *r, *s)
                    && ![p, q]
                        .iter()
                        .any(|x| x.distance(*r) < 1e-6 || x.distance(*s) < 1e-6)
            })
        })
}

/// `i` checked: its area exists on its piece, a band's edges are
/// consecutive on the outline, and it does not overlap the piece's other
/// interfacings.
fn checked(
    p: &Project,
    i: Interfacing,
    except: Option<DetailId>,
) -> Result<Interfacing, CommandError> {
    let panel = get_panel(p, i.panel)?;
    match &i.area {
        InterfacingArea::Whole => {}
        InterfacingArea::Band { edges, width_mm } => {
            finite("width_mm", &[*width_mm])?;
            if !(*width_mm > 0.0 && *width_mm <= INTERFACING_BAND_MM) {
                return Err(CommandError::invalid(
                    "width_mm",
                    Invalid::Range(0.0, INTERFACING_BAND_MM),
                ));
            }
            if edges.is_empty() {
                return Err(CommandError::invalid("edges", Invalid::Empty));
            }
            let n = panel.contour.len();
            let at: Option<Vec<usize>> = edges
                .iter()
                .map(|e| panel.contour.iter().position(|c| c == e))
                .collect();
            let joined = at.is_some_and(|at| {
                at.windows(2)
                    .all(|w| matches!(w, [a, b] if (a + 1) % n == *b))
            });
            if !joined {
                return Err(CommandError::invalid("edges", Invalid::NotJoined));
            }
        }
        InterfacingArea::Inside { line } => {
            let l = p
                .internal_line(*line)
                .ok_or_else(|| CommandError::not_found(*line))?;
            if l.panel != i.panel {
                return Err(CommandError::invalid("line", Invalid::OtherPanel));
            }
        }
    }
    let outline = p
        .interfacing_outline(&i)
        .ok_or_else(|| CommandError::invalid("area", Invalid::NotClosed))?;
    let clash = p
        .interfacings()
        .filter(|(id, o)| Some(*id) != except && o.panel == i.panel)
        .filter_map(|(_, o)| p.interfacing_outline(o))
        .any(|o| overlap(&outline, &o));
    if clash {
        return Err(CommandError::invalid("area", Invalid::Overlaps));
    }
    Ok(i)
}

/// Fuse an interfacing on a piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfacingAdd {
    pub panel: PanelId,
    /// Where: `{"kind": "whole"}`, `{"kind": "band", "edges": [...],
    /// "width_mm": 30}` or `{"kind": "inside", "line": "line:7"}`.
    /// Omitted: the whole piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<AreaSpec>,
    /// `woven`, `non_woven` or `knit` (stretches across). Omitted: woven.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabric: Option<InterfacingFabric>,
    /// `light`, `medium` or `heavy`. Omitted: medium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<InterfacingWeight>,
}

impl Described for InterfacingAdd {
    const SUMMARY: &'static str = "Fuse an interfacing on a piece: on the whole piece (default), in a band `width_mm` wide (up to 300) inside consecutive outline `edges` (a collar stand, a facing, a waistband), or inside a closed internal line. Its `fabric` (woven, non_woven, knit) and `weight` (light, medium, heavy) make the cloth there heavier, thicker and much stiffer to bend and stretch. Interfacings on one piece do not overlap. It is in the tech pack's bill of materials. Returns the interfacing (a detail).";
}

impl CommandSpec for InterfacingAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let area = match self.area.clone().unwrap_or(AreaSpec::Whole) {
            AreaSpec::Whole => InterfacingArea::Whole,
            AreaSpec::Band { edges, width_mm } => InterfacingArea::Band { edges, width_mm },
            AreaSpec::Inside { line } => InterfacingArea::Inside { line },
        };
        let i = checked(
            tx.project(),
            Interfacing {
                panel: self.panel,
                area,
                fabric: self.fabric.unwrap_or_default(),
                weight: self.weight.unwrap_or_default(),
            },
            None,
        )?;
        let id = tx.new_detail_id();
        tx.put_detail(id, Detail::Interfacing(i));
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Change an interfacing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfacingSet {
    pub interfacing: DetailId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabric: Option<InterfacingFabric>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<InterfacingWeight>,
    /// A band's width (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
}

impl Described for InterfacingSet {
    const SUMMARY: &'static str = "Change an interfacing: its `fabric`, `weight`, or a band's `width_mm`; what is omitted stays.";
}

impl CommandSpec for InterfacingSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let mut i = get_interfacing(p, self.interfacing)?;
        if let Some(w) = self.width_mm {
            match &mut i.area {
                InterfacingArea::Band { width_mm, .. } => *width_mm = w,
                _ => return Err(CommandError::invalid("width_mm", Invalid::Format)),
            }
        }
        i.fabric = self.fabric.unwrap_or(i.fabric);
        i.weight = self.weight.unwrap_or(i.weight);
        let i = checked(p, i, Some(self.interfacing))?;
        tx.put_detail(self.interfacing, Detail::Interfacing(i));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("interfacing.set:{}", self.interfacing))
    }
}

/// Remove an interfacing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfacingRemove {
    pub interfacing: DetailId,
}

impl Described for InterfacingRemove {
    const SUMMARY: &'static str = "Remove an interfacing.";
}

impl CommandSpec for InterfacingRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_interfacing(tx.project(), self.interfacing)?;
        tx.remove_detail(self.interfacing);
        Ok(CommandOutput::default())
    }
}
