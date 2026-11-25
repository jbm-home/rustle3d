//! Body measurements beyond the four a body is made from (height, bust,
//! waist, hips): neck, shoulders, arms, legs… (spec 29). Each one is the
//! value the user typed or, until then, an estimate from the four and
//! the body's masculinity.
//!
//! The estimates are linear in the four measurements, one set of
//! coefficients for a feminine body and one for a masculine body, mixed
//! by the masculinity; a child's (spec 33) have sets of their own, linear
//! in its age too, fitted the same way on 205 girls' and boys' bodies
//! (the babies', girls' and boys' size charts, and bodies whose height,
//! bust, waist or hips alone differ). The coefficients were fitted (least squares) to
//! what the built-in body model measures (`r3d_avatar::tape`) on 96
//! bodies: the women's and men's size charts at three heights, the
//! presets, and bodies whose waist, hips or bust alone differ. An
//! estimate is thus what the avatar measures (within 2.5 cm; a test in
//! `r3d-avatar` keeps them so).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::Body;

/// A body measurement besides height, bust, waist and hips.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// Around the base of the neck.
    Neck,
    /// Straight across the back from one shoulder tip to the other.
    ShoulderWidth,
    /// From the side of the neck to the shoulder tip.
    ShoulderLength,
    /// From the shoulder tip over the elbow to the wrist bone.
    ArmLength,
    /// Around the upper arm at its fullest.
    UpperArm,
    /// Around the wrist.
    Wrist,
    /// From the nape down the spine to the waist.
    BackWaistLength,
    /// From the side of the neck over the bust to the waist.
    FrontWaistLength,
    /// From the waist down to the floor.
    WaistHeight,
    /// From the hips (largest girth) down to the floor.
    HipHeight,
    /// From the waist down to the crotch (seated: to the chair).
    CrotchDepth,
    /// From the crotch down the inside of the leg to the floor.
    InsideLeg,
    /// Around the thigh at its fullest, just below the crotch.
    Thigh,
    /// Around the knee.
    Knee,
    /// Around the leg just above the ankle bone.
    Ankle,
}

/// Where a measurement is taken: how they are grouped for the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeasureGroup {
    Torso,
    Arms,
    Legs,
}

/// Linear estimate (cm): a constant, then per cm of height, bust, waist
/// and hips.
type Coefficients = [f64; 5];
/// A child's: the same, then per year of age.
type ChildCoefficients = [f64; 6];

impl Measure {
    pub const ALL: [Measure; 15] = [
        Measure::Neck,
        Measure::ShoulderWidth,
        Measure::ShoulderLength,
        Measure::BackWaistLength,
        Measure::FrontWaistLength,
        Measure::WaistHeight,
        Measure::HipHeight,
        Measure::CrotchDepth,
        Measure::ArmLength,
        Measure::UpperArm,
        Measure::Wrist,
        Measure::InsideLeg,
        Measure::Thigh,
        Measure::Knee,
        Measure::Ankle,
    ];

    /// Its position in [`Measure::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|m| *m == self)
            .unwrap_or_default()
    }

    /// Its name in formulas and commands: `arm_length`.
    pub fn name(self) -> &'static str {
        match self {
            Measure::Neck => "neck",
            Measure::ShoulderWidth => "shoulder_width",
            Measure::ShoulderLength => "shoulder_length",
            Measure::ArmLength => "arm_length",
            Measure::UpperArm => "upper_arm",
            Measure::Wrist => "wrist",
            Measure::BackWaistLength => "back_waist_length",
            Measure::FrontWaistLength => "front_waist_length",
            Measure::WaistHeight => "waist_height",
            Measure::HipHeight => "hip_height",
            Measure::CrotchDepth => "crotch_depth",
            Measure::InsideLeg => "inside_leg",
            Measure::Thigh => "thigh",
            Measure::Knee => "knee",
            Measure::Ankle => "ankle",
        }
    }

    pub fn from_name(name: &str) -> Option<Measure> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    pub fn group(self) -> MeasureGroup {
        match self {
            Measure::ArmLength | Measure::UpperArm | Measure::Wrist => MeasureGroup::Arms,
            Measure::InsideLeg | Measure::Thigh | Measure::Knee | Measure::Ankle => {
                MeasureGroup::Legs
            }
            _ => MeasureGroup::Torso,
        }
    }

    /// Whether it goes around the body (a girth) rather than along it.
    pub fn is_girth(self) -> bool {
        matches!(
            self,
            Measure::Neck
                | Measure::UpperArm
                | Measure::Wrist
                | Measure::Thigh
                | Measure::Knee
                | Measure::Ankle
        )
    }

    /// Values accepted (cm), generous enough for adults of any build and
    /// for babies.
    pub fn range_cm(self) -> (f64, f64) {
        match self {
            Measure::Neck => (18.0, 65.0),
            Measure::ShoulderWidth => (12.0, 60.0),
            Measure::ShoulderLength => (3.0, 22.0),
            Measure::ArmLength => (15.0, 85.0),
            Measure::UpperArm => (10.0, 60.0),
            Measure::Wrist => (6.0, 25.0),
            Measure::BackWaistLength => (9.0, 60.0),
            Measure::FrontWaistLength => (12.0, 65.0),
            Measure::WaistHeight => (25.0, 140.0),
            Measure::HipHeight => (20.0, 120.0),
            Measure::CrotchDepth => (9.0, 40.0),
            Measure::InsideLeg => (15.0, 100.0),
            Measure::Thigh => (16.0, 95.0),
            Measure::Knee => (11.0, 60.0),
            Measure::Ankle => (8.0, 40.0),
        }
    }

    /// Its estimate's coefficients for a feminine and a masculine body.
    fn coefficients(self) -> [Coefficients; 2] {
        match self {
            Measure::Neck => [
                [12.5334, 0.1114, 0.0399, 0.0149, 0.0157],
                [14.7700, 0.1313, 0.0049, 0.0250, 0.0116],
            ],
            Measure::ShoulderWidth => [
                [12.5251, 0.1398, 0.0005, 0.0000, -0.0004],
                [18.2311, 0.1332, -0.0011, -0.0006, 0.0026],
            ],
            Measure::ShoulderLength => [
                [4.2306, 0.0560, -0.0083, -0.0042, -0.0001],
                [7.8096, 0.0527, -0.0010, -0.0165, -0.0063],
            ],
            Measure::BackWaistLength => [
                [-2.4096, 0.1809, 0.1337, -0.0588, 0.0015],
                [22.6430, 0.1691, 0.1956, -0.4050, 0.0501],
            ],
            Measure::FrontWaistLength => [
                [3.7097, 0.1826, 0.0715, -0.0252, 0.0191],
                [22.9093, 0.1905, 0.1440, -0.3310, 0.0180],
            ],
            Measure::WaistHeight => [
                [-11.5268, 0.7057, 0.0064, 0.0125, -0.0036],
                [-30.8024, 0.7179, -0.1006, 0.3627, -0.0611],
            ],
            Measure::HipHeight => [
                [-20.6428, 0.5654, -0.0185, -0.0269, 0.1039],
                [-30.4970, 0.5741, 0.0135, -0.0175, 0.1617],
            ],
            Measure::CrotchDepth => [
                [7.9930, 0.1151, 0.0096, 0.0163, 0.0011],
                [-12.2458, 0.1122, -0.1051, 0.3887, -0.0448],
            ],
            Measure::ArmLength => [
                [-5.4643, 0.3318, 0.0070, 0.0071, -0.0074],
                [-4.1522, 0.3162, -0.0020, 0.0394, 0.0214],
            ],
            Measure::UpperArm => [
                [4.7683, 0.0172, 0.0810, 0.0593, 0.0946],
                [9.9390, 0.0610, 0.0246, 0.0912, 0.0264],
            ],
            Measure::Wrist => [
                [0.9603, 0.0714, 0.0001, -0.0007, -0.0004],
                [-0.6886, 0.0338, -0.0013, 0.0795, 0.0410],
            ],
            Measure::InsideLeg => [
                [-19.5305, 0.5907, -0.0029, -0.0039, -0.0049],
                [-18.5429, 0.6057, 0.0045, -0.0260, -0.0164],
            ],
            Measure::Thigh => [
                [-1.4687, 0.0237, 0.0330, 0.0832, 0.4397],
                [7.0804, 0.1297, 0.0164, 0.0081, 0.2470],
            ],
            Measure::Knee => [
                [12.2144, 0.0703, 0.0133, 0.0506, 0.0527],
                [12.1525, 0.0856, 0.0073, 0.0640, 0.0283],
            ],
            Measure::Ankle => [
                [-2.7533, 0.1299, 0.0008, -0.0042, -0.0025],
                [2.3785, 0.1117, 0.0005, 0.0004, 0.0020],
            ],
        }
    }

    /// Its estimate's coefficients for a girl's and a boy's body: as
    /// [`Measure::coefficients`], then per year of age.
    fn child_coefficients(self) -> [ChildCoefficients; 2] {
        match self {
            Measure::Neck => [
                [39.2551, -0.1000, -0.0445, -0.0739, -0.0748, 2.4355],
                [26.9326, -0.0287, 0.0397, -0.0491, 0.0061, 1.0870],
            ],
            Measure::ShoulderWidth => [
                [11.5714, 0.1001, -0.0188, -0.0133, -0.0075, 0.5773],
                [13.8374, 0.1148, -0.0653, -0.0060, -0.0445, 1.0330],
            ],
            Measure::ShoulderLength => [
                [-0.6764, 0.0671, 0.0021, 0.0053, 0.0112, -0.1257],
                [2.6992, 0.0627, -0.0361, 0.0030, -0.0230, 0.3613],
            ],
            Measure::BackWaistLength => [
                [0.8258, 0.1011, 0.3105, -0.1089, -0.0425, 0.1146],
                [4.3198, 0.1188, 0.2256, -0.1149, -0.0654, 0.6814],
            ],
            Measure::FrontWaistLength => [
                [2.8898, 0.0862, 0.1922, -0.0208, 0.0195, 0.1639],
                [6.7463, 0.1201, 0.1548, -0.0561, -0.0521, 0.7110],
            ],
            Measure::WaistHeight => [
                [-11.6898, 0.7878, -0.1159, 0.0641, 0.0350, -0.5551],
                [-14.5790, 0.7552, -0.0651, 0.0768, 0.0911, -1.0561],
            ],
            Measure::HipHeight => [
                [-7.5982, 0.4674, 0.0272, 0.2824, -0.0462, -0.4297],
                [-10.0772, 0.4262, 0.0427, -0.0232, 0.3403, -0.7564],
            ],
            Measure::CrotchDepth => [
                [5.3024, 0.1482, -0.1255, 0.0544, 0.0300, 0.1286],
                [3.9535, 0.1359, -0.1208, 0.0790, 0.0549, -0.1865],
            ],
            Measure::ArmLength => [
                [-1.6584, 0.3564, 0.0022, 0.0004, -0.0027, -0.3247],
                [-3.9296, 0.3359, 0.0498, -0.0039, 0.0378, -0.5298],
            ],
            Measure::UpperArm => [
                [10.5489, 0.0392, 0.0058, -0.0231, 0.0304, 0.4229],
                [12.6143, 0.0455, -0.0318, 0.0083, -0.0173, 0.7685],
            ],
            Measure::Wrist => [
                [2.3564, 0.0628, 0.0219, -0.0030, 0.0143, -0.2195],
                [1.9250, 0.0523, 0.0278, 0.0097, 0.0212, -0.1849],
            ],
            Measure::InsideLeg => [
                [-16.9922, 0.6396, 0.0096, 0.0097, 0.0051, -0.6838],
                [-18.5325, 0.6192, 0.0556, -0.0023, 0.0362, -0.8696],
            ],
            Measure::Thigh => [
                [7.7710, 0.1053, -0.0327, -0.0891, 0.2870, 0.5466],
                [11.4514, 0.1138, -0.0347, -0.0588, 0.1664, 0.7589],
            ],
            Measure::Knee => [
                [11.2743, 0.0764, -0.0140, -0.0139, 0.0109, 0.5887],
                [12.8587, 0.0838, -0.0440, 0.0080, -0.0273, 0.7801],
            ],
            Measure::Ankle => [
                [4.1930, 0.1030, 0.0112, -0.0109, 0.0052, -0.1437],
                [4.7453, 0.1017, 0.0042, -0.0134, 0.0018, -0.0004],
            ],
        }
    }

    /// Its estimate for `body` (cm), within its range.
    pub fn estimate(self, body: &Body) -> f64 {
        let linear = |[k, h, b, w, hp]: Coefficients| {
            k + h * body.height_cm + b * body.bust_cm + w * body.waist_cm + hp * body.hips_cm
        };
        let m = body.masculinity.clamp(0.0, 1.0);
        let v = match body.age_years {
            None => {
                let [feminine, masculine] = self.coefficients();
                (1.0 - m) * linear(feminine) + m * linear(masculine)
            }
            Some(age) => {
                let aged =
                    |[k, h, b, w, hp, a]: ChildCoefficients| linear([k, h, b, w, hp]) + a * age;
                let [girl, boy] = self.child_coefficients();
                (1.0 - m) * aged(girl) + m * aged(boy)
            }
        };
        let (min, max) = self.range_cm();
        (v * 10.0).round().clamp(min * 10.0, max * 10.0) / 10.0
    }
}

/// The measurements the user typed (cm); the others are estimated.
pub type Measures = BTreeMap<Measure, f64>;

/// One measurement of a body: its value and whether it is estimated.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeasureValue {
    pub cm: f64,
    pub estimated: bool,
}

/// Every measurement of `body` given those typed: the typed value, or
/// else the estimate.
pub fn value(body: &Body, typed: &Measures, m: Measure) -> MeasureValue {
    match typed.get(&m) {
        Some(cm) => MeasureValue {
            cm: *cm,
            estimated: false,
        },
        None => MeasureValue {
            cm: m.estimate(body),
            estimated: true,
        },
    }
}

/// The typed measurements of another body (a size of a range) whose four
/// measurements are `to`'s while those typed were for `from`'s: each
/// moves by as much as its estimate does, so that the user's own neck
/// stays theirs, only graded.
pub fn regraded(typed: &Measures, from: &Body, to: &Body) -> Measures {
    typed
        .iter()
        .map(|(m, cm)| {
            let (min, max) = m.range_cm();
            let v = cm + m.estimate(to) - m.estimate(from);
            (*m, v.clamp(min, max))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_read_back() {
        for m in Measure::ALL {
            assert_eq!(Measure::from_name(m.name()), Some(m));
            assert_eq!(Measure::ALL[m.index()], m);
        }
        let json = serde_json::to_string(&Measure::ArmLength).unwrap_or_default();
        assert_eq!(json, "\"arm_length\"");
    }

    #[test]
    fn a_typed_value_wins_and_regrades_with_its_estimate() {
        let body = Body {
            masculinity: 0.0,
            height_cm: 168.0,
            bust_cm: 90.0,
            waist_cm: 73.0,
            hips_cm: 98.0,
            age_years: None,
        };
        let typed = Measures::from([(Measure::Neck, 40.0)]);
        assert_eq!(
            value(&body, &typed, Measure::Neck),
            MeasureValue {
                cm: 40.0,
                estimated: false
            }
        );
        assert!(value(&body, &typed, Measure::Knee).estimated);
        let bigger = Body {
            bust_cm: 98.0,
            ..body
        };
        let moved = Measure::Neck.estimate(&bigger) - Measure::Neck.estimate(&body);
        let graded = regraded(&typed, &body, &bigger);
        assert!((graded[&Measure::Neck] - (40.0 + moved)).abs() < 1e-9);
    }
}
