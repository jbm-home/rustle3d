//! A project that makes an object rather than a garment worn (spec 36):
//! a bag, a cushion, a toy. It has no body; its drape is weightless, so
//! the cloth keeps the shape its seams give it instead of falling in
//! folds, and its closed parts can be stuffed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The stuffing's range: 0 none, 1 firm.
pub const STUFFING: std::ops::RangeInclusive<f64> = 0.0..=1.0;

/// How an object project is shown in 3D.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObjectSetup {
    /// How firmly the parts closed all around (a cushion, a toy) are
    /// stuffed: 0 none (the cloth keeps its sewn shape), 1 firm. Parts
    /// left open (a bag) are never stuffed.
    pub stuffing: f64,
}

impl ObjectSetup {
    /// A new object project's: lightly stuffed, so a cushion is round
    /// and smooth without straining its cloth.
    pub const DEFAULT: ObjectSetup = ObjectSetup { stuffing: 0.5 };

    /// Whether every number is in range.
    pub fn valid(&self) -> bool {
        STUFFING.contains(&self.stuffing)
    }
}
