//! The photo studio: a seamless backdrop (a floor curving up into a round
//! wall, like a photographer's cyclorama), rectangular soft boxes placed
//! around the garment from where the camera looks, and a dim sky above
//! the backdrop.

use glam::{Quat, Vec3};

use super::Lighting;

/// A one-sided rectangular light: `corner`, its two (perpendicular) edges,
/// and the radiance it emits on the side of `edge_u × edge_v`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AreaLight {
    pub corner: Vec3,
    pub edge_u: Vec3,
    pub edge_v: Vec3,
    pub radiance: Vec3,
}

impl AreaLight {
    /// A `width × height` soft box at `at`, facing `target`.
    fn facing(at: Vec3, target: Vec3, width: f32, height: f32, radiance: f32) -> Self {
        let normal = (target - at).normalize_or(Vec3::NEG_Z);
        let side = Vec3::Y.cross(normal).try_normalize().unwrap_or(Vec3::X);
        let up = normal.cross(side);
        // u × v must point along the normal: side × up = normal.
        let (u, v) = (side * width, up * height);
        Self {
            corner: at - u * 0.5 - v * 0.5,
            edge_u: u,
            edge_v: v,
            radiance: Vec3::splat(radiance),
        }
    }

    pub fn area(&self) -> f32 {
        self.edge_u.cross(self.edge_v).length()
    }

    /// Emitted power, up to a constant (to pick lights for sampling).
    pub fn power(&self) -> f32 {
        self.area() * (self.radiance.x + self.radiance.y + self.radiance.z)
    }
}

/// The lights, sky and backdrop of a lighting setup.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Studio {
    pub lights: Vec<AreaLight>,
    /// Sky radiance straight up and at the horizon (linear RGB).
    pub sky_top: Vec3,
    pub sky_horizon: Vec3,
    /// Backdrop reflectance (linear).
    pub backdrop: Vec3,
}

impl Studio {
    /// `lighting` around a subject centred at `centre`, seen from `eye`:
    /// the soft boxes keep their place relative to the camera, as a
    /// photographer moves them when walking around the model, turned by
    /// `turn` (radians, counter-clockwise seen from above). `brightness`
    /// scales the lights and the sky.
    pub fn new(lighting: Lighting, centre: Vec3, eye: Vec3, turn: f32, brightness: f32) -> Self {
        let toward = Quat::from_rotation_y(turn)
            * Vec3::new(eye.x - centre.x, 0.0, eye.z - centre.z)
                .try_normalize()
                .unwrap_or(Vec3::Z);
        let right = Vec3::Y.cross(toward);
        // A point `front` metres toward the camera, `side` to its right,
        // `up` above the centre.
        let at =
            |front: f32, side: f32, up: f32| centre + toward * front + right * side + Vec3::Y * up;
        let aim = centre + Vec3::Y * 0.1;
        let mut studio = match lighting {
            Lighting::Studio => Self {
                lights: vec![
                    // Key: a large box high on the left.
                    AreaLight::facing(at(2.2, -1.8, 1.4), aim, 1.4, 1.4, 16.0),
                    // Fill: a tall strip on the right, low.
                    AreaLight::facing(at(2.4, 2.2, 0.1), aim, 1.0, 1.8, 3.2),
                    // Rim: behind, to separate the garment from the backdrop.
                    AreaLight::facing(at(-2.2, 1.0, 1.0), aim, 0.6, 1.8, 9.0),
                ],
                sky_top: Vec3::splat(0.25),
                sky_horizon: Vec3::splat(0.15),
                backdrop: Vec3::splat(0.75),
            },
            Lighting::Soft => Self {
                lights: vec![
                    AreaLight::facing(at(2.0, -1.9, 0.6), aim, 2.0, 2.0, 4.0),
                    AreaLight::facing(at(2.0, 1.9, 0.6), aim, 2.0, 2.0, 4.0),
                    AreaLight::facing(at(0.6, 0.0, 2.4), aim, 2.0, 2.0, 3.0),
                ],
                sky_top: Vec3::splat(0.9),
                sky_horizon: Vec3::splat(0.7),
                backdrop: Vec3::splat(0.85),
            },
            Lighting::Dramatic => Self {
                lights: vec![
                    AreaLight::facing(at(0.9, -2.6, 1.2), aim, 0.7, 1.0, 40.0),
                    AreaLight::facing(at(-2.0, 1.6, 0.9), aim, 0.4, 1.6, 26.0),
                ],
                sky_top: Vec3::splat(0.02),
                sky_horizon: Vec3::splat(0.01),
                backdrop: Vec3::new(0.05, 0.05, 0.055),
            },
        };
        for l in &mut studio.lights {
            l.radiance *= brightness;
        }
        studio.sky_top *= brightness;
        studio.sky_horizon *= brightness;
        studio
    }
}

/// The backdrop: a floor (y = 0) of radius `radius - BEND`, curving up
/// into a round wall `height` high. Triangles with inward normals.
pub(crate) fn backdrop(radius: f32, height: f32) -> Vec<([Vec3; 3], [Vec3; 3])> {
    const AROUND: usize = 96;
    const BEND: f32 = 2.0;
    // Profile from the centre outwards: (distance from the axis, height,
    // inward normal in the profile plane as (radial, up)).
    let floor = radius - BEND;
    let mut profile: Vec<(f32, f32, f32, f32)> = vec![(0.0, 0.0, 0.0, 1.0)];
    for i in 1..=6 {
        profile.push((floor * i as f32 / 6.0, 0.0, 0.0, 1.0));
    }
    for i in 1..=12 {
        let a = std::f32::consts::FRAC_PI_2 * i as f32 / 12.0;
        let (s, c) = a.sin_cos();
        // Circle of radius BEND centred at (floor, BEND).
        profile.push((floor + BEND * s, BEND * (1.0 - c), -s, c));
    }
    let wall_bottom = BEND;
    for i in 1..=4 {
        let y = wall_bottom + (height - wall_bottom).max(0.0) * i as f32 / 4.0;
        profile.push((radius, y, -1.0, 0.0));
    }
    let point = |k: usize, a: usize| -> (Vec3, Vec3) {
        let (r, y, nr, ny) = profile[k];
        let angle = std::f32::consts::TAU * a as f32 / AROUND as f32;
        let (s, c) = angle.sin_cos();
        (
            Vec3::new(r * c, y, r * s),
            Vec3::new(nr * c, ny, nr * s).normalize_or(Vec3::Y),
        )
    };
    let mut out = Vec::new();
    for k in 0..profile.len() - 1 {
        for a in 0..AROUND {
            let (p00, n00) = point(k, a);
            let (p01, n01) = point(k, a + 1);
            let (p10, n10) = point(k + 1, a);
            let (p11, n11) = point(k + 1, a + 1);
            if k > 0 {
                out.push(([p00, p01, p10], [n00, n01, n10]));
            }
            out.push(([p01, p11, p10], [n01, n11, n10]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_boxes_face_the_subject() {
        let centre = Vec3::new(0.0, 1.0, 0.0);
        for lighting in [Lighting::Studio, Lighting::Soft, Lighting::Dramatic] {
            let studio = Studio::new(lighting, centre, Vec3::new(0.0, 1.0, 3.0), 0.0, 1.0);
            for l in &studio.lights {
                let normal = l.edge_u.cross(l.edge_v).normalize();
                let middle = l.corner + (l.edge_u + l.edge_v) * 0.5;
                assert!(normal.dot(centre - middle) > 0.0);
                assert!(l.area() > 0.1 && l.power() > 0.0);
                // Inside the backdrop.
                assert!(Vec3::new(middle.x, 0.0, middle.z).length() < 4.5);
            }
        }
    }

    #[test]
    fn the_studio_turns_with_the_camera() {
        let c = Vec3::ZERO;
        let front = Studio::new(Lighting::Studio, c, Vec3::Z * 3.0, 0.0, 1.0);
        let back = Studio::new(Lighting::Studio, c, -Vec3::Z * 3.0, 0.0, 1.0);
        let (a, b) = (front.lights[0].corner, back.lights[0].corner);
        assert!((a.x + b.x).abs() < 1e-5 && (a.z + b.z).abs() < 1e-5);
        // Turning the lights half a turn puts them where the camera
        // behind would have them, twice as bright.
        let turned = Studio::new(
            Lighting::Studio,
            c,
            Vec3::Z * 3.0,
            std::f32::consts::PI,
            2.0,
        );
        assert!(turned.lights[0].corner.distance(b) < 1e-4);
        assert_eq!(turned.lights[0].radiance, back.lights[0].radiance * 2.0);
    }

    #[test]
    fn the_backdrop_is_a_bowl_seen_from_inside() {
        let tris = backdrop(6.0, 5.0);
        assert!(!tris.is_empty());
        for (p, n) in &tris {
            for (v, nv) in p.iter().zip(n) {
                assert!(v.y >= -1e-6 && v.y <= 5.0 + 1e-4);
                // Normals face the axis or up.
                let inward = Vec3::new(-v.x, 0.0, -v.z);
                assert!(nv.y > -1e-6 && (nv.dot(inward) >= -1e-4));
            }
        }
    }
}
