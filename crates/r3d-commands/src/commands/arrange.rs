//! Place every panel around the body ("Arrange all").

use r3d_core::{Around, PanelId, Placement, Project, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::centre;
use crate::error::{CommandError, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Panel tops when there is no avatar (cm).
const DEFAULT_TOP_CM: f64 = 110.0;

/// Height of the avatar's waist (its narrowest girth) above the floor
/// (cm); [`DEFAULT_TOP_CM`] without an avatar. Skirts hang from there.
pub(crate) fn waist_height_cm(p: &Project) -> Result<f64, CommandError> {
    let Some(avatar) = p.avatar() else {
        return Ok(DEFAULT_TOP_CM);
    };
    let fit = crate::queries::fitted(&avatar.body)?;
    Ok(f64::from(fit.achieved.waist_height_cm))
}

/// Place every panel around the body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GarmentArrange {
    /// Height of the panels' tops (cm). Default: the avatar's waist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_cm: Option<f64>,
}

impl Described for GarmentArrange {
    const SUMMARY: &'static str = "Place every panel around the body for the simulation, side by side in the order the seams join them (a panel's right side continues into the next panel; each takes a share of the turn in proportion to its width), tops at the avatar's waist (its narrowest girth) unless top_cm is given. Panels placed on an arm (sleeves) or flat keep their placement. Re-run it after adding panels or seams.";
}

fn off_body(p: &Project, panel: PanelId) -> bool {
    p.panel(panel)
        .and_then(|pn| pn.placement)
        .is_some_and(|pl| pl.around != Around::Body)
}

/// The body panels in the order the seams join them: from a panel, the
/// next one is sewn to its right side (larger x on the pattern).
fn ring(p: &Project) -> Vec<PanelId> {
    let mut left: Vec<PanelId> = p
        .panels()
        .keys()
        .copied()
        .filter(|id| !off_body(p, *id))
        .collect();
    let mut order = Vec::with_capacity(left.len());
    let mut current = left.first().copied();
    while let Some(panel) = current {
        left.retain(|x| *x != panel);
        order.push(panel);
        let mid_x = centre(p, panel).x;
        let right_neighbour = p.panel(panel).and_then(|pn| {
            pn.contour.iter().find_map(|e| {
                let seam = p.seams().get(&p.seam_of(*e)?)?;
                let (_, (other, _)) = seam.across(*e)?;
                let other_panel = p.edge(other)?.panel;
                let line = p.edge_polyline(*e)?;
                let x = line.iter().map(|v| v.x).sum::<f64>() / line.len().max(1) as f64;
                (x > mid_x && left.contains(&other_panel)).then_some(other_panel)
            })
        });
        current = right_neighbour.or_else(|| left.first().copied());
    }
    order
}

/// Placement angles (degrees) for body panels side by side around the
/// body, in this order: each takes a share of the turn in proportion to
/// its width, the first centred at 0°. Spaced evenly instead, a wide
/// front and two narrow backs left a gap at the centre back that the seam
/// closed straight across, between the legs.
fn side_by_side(p: &Project, order: &[PanelId]) -> Vec<f64> {
    let widths: Vec<f64> = order
        .iter()
        .map(|id| {
            p.panel_bounding_box(*id)
                .map_or(0.0, |b| (b.max.x - b.min.x).max(0.0))
        })
        .collect();
    let total: f64 = widths.iter().sum();
    if total <= 0.0 {
        let n = order.len().max(1) as f64;
        return (0..order.len()).map(|i| 360.0 * i as f64 / n).collect();
    }
    let first = widths.first().copied().unwrap_or_default();
    let mut before = 0.0;
    widths
        .iter()
        .map(|w| {
            let angle = 360.0 * (before + w / 2.0 - first / 2.0) / total;
            before += w;
            angle
        })
        .collect()
}

impl CommandSpec for GarmentArrange {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let top_cm = match self.top_cm {
            Some(t) => {
                positive("top_cm", t)?;
                t
            }
            None => waist_height_cm(p)?,
        };
        let order = ring(p);
        let panels: Vec<_> = order
            .iter()
            .zip(side_by_side(p, &order))
            .filter_map(|(id, angle)| Some((*id, p.panel(*id)?.clone(), angle)))
            .collect();
        for (id, mut panel, angle_deg) in panels {
            panel.placement = Some(Placement {
                around: Around::Body,
                angle_deg,
                top_cm,
                offset_mm: panel.placement.map_or(30.0, |pl| pl.offset_mm),
                centre_x_mm: None,
            });
            tx.put_panel(id, panel);
        }
        Ok(CommandOutput::default())
    }
}
