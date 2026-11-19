//! Trims: buttons and zippers (`docs/specs/26-trims.md`). Both are
//! closures: a button through its buttonhole holds two points of the
//! garment together; a zipper joins two stretches of outline from its
//! bottom stop up to its slider. Open, they hold nothing.

use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::PanelId;
use crate::model::{Project, SeamSide};

/// A buttonhole is this much longer than its button is wide (mm): room
/// for the button's thickness (general sewing practice).
pub const BUTTONHOLE_EXTRA_MM: f64 = 3.0;

/// A button sewn on a piece and the buttonhole it goes through: a closure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    /// The piece the button is sewn on.
    pub panel: PanelId,
    /// Where it is sewn (mm, board coordinates).
    pub at: DVec2,
    /// Its diameter (mm, > 0).
    pub diameter_mm: f64,
    pub hole: Buttonhole,
    /// Buttoned: the button point and the buttonhole's end are held
    /// together in the drape.
    pub closed: bool,
}

/// Where a button goes through: a slit on a piece (usually the one that
/// overlaps the button's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buttonhole {
    pub panel: PanelId,
    /// The slit's end where the button sits once buttoned (mm, board
    /// coordinates): on a front, the end nearest the edge.
    pub at: DVec2,
    /// Direction of the slit from `at` (degrees from the board's +X).
    pub angle_deg: f64,
}

impl Button {
    /// The buttonhole's slit on the pattern: from where the button sits to
    /// the other end, the button's diameter plus [`BUTTONHOLE_EXTRA_MM`].
    pub fn slit(&self) -> [DVec2; 2] {
        let dir = DVec2::from_angle(self.hole.angle_deg.to_radians());
        let len = self.diameter_mm + BUTTONHOLE_EXTRA_MM;
        [self.hole.at, self.hole.at + dir * len]
    }
}

/// A zipper joining two stretches of outline (or internal lines), point
/// for point along their lengths like a seam, from their starts (the
/// bottom stop) up to the slider; open above it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Zipper {
    pub a: SeamSide,
    pub b: SeamSide,
    /// How much of it is open, from its top end down to the slider (mm,
    /// ≥ 0). 0: closed; its length or more: open all the way. Measured
    /// from the top so a closed zipper stays closed when its sides change
    /// length.
    pub open_mm: f64,
}

impl Project {
    /// A zipper's length: its shorter side's (mm).
    pub fn zipper_length(&self, z: &Zipper) -> Option<f64> {
        Some(self.side_length(&z.a)?.min(self.side_length(&z.b)?))
    }

    /// The share of a zipper that is closed, from its bottom stop (0–1).
    pub fn zipper_closed(&self, z: &Zipper) -> f64 {
        match self.zipper_length(z) {
            Some(len) if len > 0.0 => (1.0 - z.open_mm / len).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slit_runs_from_the_button_end() {
        let b = Button {
            panel: PanelId::from_raw(1),
            at: DVec2::ZERO,
            diameter_mm: 11.0,
            hole: Buttonhole {
                panel: PanelId::from_raw(2),
                at: DVec2::new(10.0, 20.0),
                angle_deg: 90.0,
            },
            closed: true,
        };
        let [a, z] = b.slit();
        assert_eq!(a, DVec2::new(10.0, 20.0));
        assert!((z - DVec2::new(10.0, 34.0)).length() < 1e-9);
    }
}
