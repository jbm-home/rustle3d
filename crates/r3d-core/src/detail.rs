//! Construction details that change the cloth where they are
//! (`docs/specs/31-construction-details-2.md`): elastic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::id::PanelId;
use crate::model::{Project, SeamSide};

/// The smallest and largest elastic ratios (relaxed length over the
/// side's length).
pub const ELASTIC_RATIO: (f64, f64) = (0.2, 1.0);
/// The narrowest and widest elastics (mm).
pub const ELASTIC_WIDTH_MM: (f64, f64) = (2.0, 50.0);
/// How hard an elastic pulls: newtons per unit strain per millimetre of
/// width (≈ 0.8 N per mm when stretched to twice its length; general
/// knowledge of knitted and braided elastics, an estimate to calibrate,
/// docs/physics.md "Elastic").
pub const ELASTIC_N_PER_MM: f64 = 0.8;

/// A detail on a piece: one entity kind for what spec 31 adds, each its
/// own variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Detail {
    /// An elastic along a side, gathering it.
    Elastic(Elastic),
}

/// An elastic sewn along a side (an edge, consecutive outline edges or an
/// internal line, maybe a stretch of it), stretched to its length: the
/// side gathers to `ratio` times its length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Elastic {
    pub side: SeamSide,
    /// Relaxed length over the side's length (0.2–1).
    pub ratio: f64,
    /// Its width (mm): how strongly it pulls.
    pub width_mm: f64,
}

impl Elastic {
    /// Its stiffness: newtons per unit strain.
    pub fn stiffness_n(&self) -> f64 {
        ELASTIC_N_PER_MM * self.width_mm
    }

    /// Ratio and width within their ranges.
    pub fn valid(&self) -> bool {
        (ELASTIC_RATIO.0..=ELASTIC_RATIO.1).contains(&self.ratio)
            && (ELASTIC_WIDTH_MM.0..=ELASTIC_WIDTH_MM.1).contains(&self.width_mm)
    }
}

impl Detail {
    /// The piece it is on.
    pub fn panel(&self, p: &Project) -> Option<PanelId> {
        match self {
            Detail::Elastic(e) => p.side_panel(&e.side),
        }
    }
}

impl Project {
    /// Every elastic.
    pub fn elastics(&self) -> impl Iterator<Item = (crate::id::DetailId, &Elastic)> {
        self.details.iter().map(|(id, d)| match d {
            Detail::Elastic(e) => (*id, e),
        })
    }

    /// An elastic's relaxed length (mm): its side's times its ratio.
    pub fn elastic_length(&self, e: &Elastic) -> Option<f64> {
        Some(self.side_length(&e.side)? * e.ratio)
    }
}
