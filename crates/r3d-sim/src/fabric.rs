//! Fabric parameters and presets (docs/physics.md §3).

use serde::{Deserialize, Serialize};

/// Physical parameters of a fabric, in SI units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FabricParams {
    /// Areal density (kg/m²).
    pub density: f32,
    /// Collision thickness (m).
    pub thickness: f32,
    /// In-plane stiffness along the warp (N/m).
    pub stretch_warp: f32,
    /// In-plane stiffness along the weft (N/m).
    pub stretch_weft: f32,
    /// In-plane shear stiffness (N/m).
    pub shear: f32,
    /// Bending rigidity (N·m).
    pub bending: f32,
    /// Coulomb friction against the body (0–1).
    pub friction: f32,
    /// Velocity damping per second (0–1).
    pub damping: f32,
}

/// Topstitched cloth bends this much harder: the thread and the seam
/// allowance under it (docs/physics.md, "Composite cloth").
pub const TOPSTITCH_BENDING: f32 = 1.5;

impl FabricParams {
    /// The cloth under topstitching.
    pub fn topstitched(self) -> Self {
        Self {
            bending: self.bending * TOPSTITCH_BENDING,
            ..self
        }
    }
}

/// The built-in fabrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FabricPreset {
    Cotton,
    Denim,
    Jersey,
    Silk,
    Wool,
}

impl FabricPreset {
    pub const ALL: [FabricPreset; 5] = [
        FabricPreset::Cotton,
        FabricPreset::Denim,
        FabricPreset::Jersey,
        FabricPreset::Silk,
        FabricPreset::Wool,
    ];

    /// Initial estimates from general textile knowledge (fabric weights,
    /// typical bending and shear rigidities); to be calibrated
    /// (docs/physics.md §3). Shear is an order below tension, as in
    /// wovens: stiffer shear kept the triangles from bending smoothly and
    /// the cloth set in crumples.
    pub fn params(self) -> FabricParams {
        // (g/m², mm, warp N/m, weft N/m, shear N/m, bending µN·m, friction)
        let (gsm, mm, warp, weft, shear, bend, friction) = match self {
            FabricPreset::Cotton => (120.0, 0.30, 8000.0, 6000.0, 40.0, 30.0, 0.40),
            FabricPreset::Denim => (400.0, 0.80, 20000.0, 15000.0, 150.0, 60.0, 0.50),
            FabricPreset::Jersey => (180.0, 0.60, 300.0, 150.0, 10.0, 4.0, 0.50),
            FabricPreset::Silk => (80.0, 0.15, 5000.0, 4000.0, 15.0, 1.5, 0.25),
            FabricPreset::Wool => (260.0, 0.60, 10000.0, 8000.0, 60.0, 20.0, 0.45),
        };
        FabricParams {
            density: gsm / 1000.0,
            thickness: mm / 1000.0,
            stretch_warp: warp,
            stretch_weft: weft,
            shear,
            bending: bend * 1e-6,
            friction,
            // Mostly air drag: light cloth settles within a couple of seconds.
            damping: 1.0,
        }
    }
}
