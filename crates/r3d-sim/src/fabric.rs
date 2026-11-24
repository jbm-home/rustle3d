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

/// How an interfacing is made (docs/physics.md, "Composite cloth").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterfacingMake {
    Woven,
    NonWoven,
    Knit,
}

/// How heavy an interfacing is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterfacingWeight {
    Light,
    Medium,
    Heavy,
}

/// Fused layers bend about a shared neutral axis: their rigidity is this
/// many times the sum of theirs (a plate's t³ law gives 4 for two equal
/// layers; our estimate, to calibrate).
pub const FUSED_BENDING: f32 = 3.0;

/// An interfacing on its own: weight, thickness, stretch (warp, weft),
/// shear and bending, in SI units. Estimates from general textile
/// knowledge (docs/physics.md, "Composite cloth").
pub fn interfacing(make: InterfacingMake, weight: InterfacingWeight) -> FabricParams {
    // (g/m², mm, stretch N/m, woven shear N/m, non-woven shear N/m, µN·m)
    let (gsm, mm, stretch, woven_shear, felt_shear, bend) = match weight {
        InterfacingWeight::Light => (25.0, 0.15, 3000.0, 20.0, 100.0, 10.0),
        InterfacingWeight::Medium => (45.0, 0.25, 5000.0, 40.0, 200.0, 40.0),
        InterfacingWeight::Heavy => (80.0, 0.40, 8000.0, 80.0, 400.0, 150.0),
    };
    let (weft, shear) = match make {
        InterfacingMake::Woven => (stretch, woven_shear),
        InterfacingMake::NonWoven => (stretch, felt_shear),
        InterfacingMake::Knit => (stretch / 20.0, woven_shear / 4.0),
    };
    FabricParams {
        density: gsm / 1000.0,
        thickness: mm / 1000.0,
        stretch_warp: stretch,
        stretch_weft: weft,
        shear,
        bending: bend * 1e-6,
        friction: 0.0,
        damping: 0.0,
    }
}

impl FabricParams {
    /// The cloth with `layer` fused to it: weights, thicknesses and
    /// in-plane stiffnesses add; bending is [`FUSED_BENDING`] times the
    /// sum. Friction and damping stay the cloth's.
    pub fn fused(self, layer: FabricParams) -> Self {
        Self {
            density: self.density + layer.density,
            thickness: self.thickness + layer.thickness,
            stretch_warp: self.stretch_warp + layer.stretch_warp,
            stretch_weft: self.stretch_weft + layer.stretch_weft,
            shear: self.shear + layer.shear,
            bending: FUSED_BENDING * (self.bending + layer.bending),
            ..self
        }
    }
}

/// A quilting's padding bends this much per square millimetre of its
/// loft (N·m): with 8 mm on cotton, a bending length of about 7 cm (our
/// estimate, docs/physics.md "Composite cloth").
pub const PADDING_BENDING_PER_MM2: f32 = 15e-6;

impl FabricParams {
    /// The cloth quilted over a padding `loft_mm` thick weighing
    /// `padding_gsm`, with a lining like itself: one layer standing for
    /// the three.
    pub fn quilted(self, loft_mm: f32, padding_gsm: f32) -> Self {
        Self {
            density: 2.0 * self.density + padding_gsm / 1000.0,
            thickness: self.thickness + loft_mm / 1000.0,
            stretch_warp: 2.0 * self.stretch_warp,
            stretch_weft: 2.0 * self.stretch_weft,
            shear: 2.0 * self.shear,
            bending: 2.0 * self.bending + PADDING_BENDING_PER_MM2 * loft_mm * loft_mm,
            ..self
        }
    }

    /// Bending along a quilting's stitched lines: the shell and the
    /// lining only.
    pub fn quilted_crease(self) -> f32 {
        2.0 * self.bending
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
