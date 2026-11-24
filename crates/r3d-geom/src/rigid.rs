//! Rigid motions in 3D (`f32`, metres): how a bone of a moving body
//! carries the points attached to it, and how a skin vertex follows
//! several bones ([`Rigid::blend`]).

use glam::{Quat, Vec3, Vec4};

/// The rigid motion `p ↦ rotation · p + translation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    pub rotation: Quat,
    pub translation: Vec3,
}

impl Default for Rigid {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rigid {
    pub const IDENTITY: Self = Self {
        rotation: Quat::IDENTITY,
        translation: Vec3::ZERO,
    };

    /// Turning by `rotation` about the point `pivot`.
    pub fn about(pivot: Vec3, rotation: Quat) -> Self {
        Self {
            rotation,
            translation: pivot - rotation * pivot,
        }
    }

    pub fn apply(&self, p: Vec3) -> Vec3 {
        self.rotation * p + self.translation
    }

    /// The motion that undoes this one.
    pub fn inverse(&self) -> Self {
        let rotation = self.rotation.inverse();
        Self {
            rotation,
            translation: -(rotation * self.translation),
        }
    }

    /// `self` after `first`: `p ↦ self(first(p))`.
    pub fn after(&self, first: &Self) -> Self {
        Self {
            rotation: (self.rotation * first.rotation).normalize(),
            translation: self.rotation * first.translation + self.translation,
        }
    }

    /// The weighted blend of rigid motions, itself rigid: dual quaternion
    /// blending (Kavan, Collins, Žára, O'Sullivan, "Geometric skinning with
    /// approximate dual quaternion blending", ACM Transactions on Graphics
    /// 27(4), 2008). Each motion is the unit dual quaternion
    /// `r + ε ½ t r`; they are summed by weight, each turned to the
    /// hemisphere of the heaviest one's rotation, and the sum normalised.
    /// Unlike averaging the moved points (linear blend skinning), a joint
    /// bent far keeps its volume: no collapsed elbow, no folded armpit.
    /// `None` when no weight is positive.
    pub fn blend(parts: &[(Self, f32)]) -> Option<Self> {
        let pivot = parts
            .iter()
            .filter(|(_, w)| *w > 0.0)
            .fold(None::<(Quat, f32)>, |best, (r, w)| match best {
                Some((_, b)) if b >= *w => best,
                _ => Some((r.rotation, *w)),
            })?
            .0;
        let mut real = Vec4::ZERO;
        let mut dual = Vec4::ZERO;
        for (r, w) in parts.iter().filter(|(_, w)| *w > 0.0) {
            let w = if r.rotation.dot(pivot) < 0.0 { -*w } else { *w };
            let q = Vec4::from(r.rotation);
            real += q * w;
            dual += Vec4::from(
                Quat::from_xyzw(r.translation.x, r.translation.y, r.translation.z, 0.0)
                    * r.rotation,
            ) * (0.5 * w);
        }
        let n = real.length();
        if n.is_nan() || n < 1e-6 {
            return None;
        }
        let (r, d) = (real / n, dual / n);
        // t = 2 d r̄ (its vector part).
        let (rv, dv) = (r.truncate(), d.truncate());
        let translation = 2.0 * (r.w * dv - d.w * rv + rv.cross(dv));
        Some(Self {
            rotation: Quat::from_vec4(r).normalize(),
            translation,
        })
    }

    /// Between `self` (at 0) and `other` (at 1): rotations by spherical
    /// interpolation, translations linearly.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            rotation: self.rotation.slerp(other.rotation, t),
            translation: self.translation.lerp(other.translation, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-5
    }

    #[test]
    fn about_after_inverse_and_lerp() {
        let pivot = Vec3::new(1.0, 2.0, 0.0);
        let quarter = Rigid::about(pivot, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        assert!(close(quarter.apply(pivot), pivot));
        assert!(close(quarter.apply(pivot + Vec3::X), pivot + Vec3::Y));
        let shift = Rigid {
            rotation: Quat::IDENTITY,
            translation: Vec3::Z,
        };
        let p = Vec3::new(0.3, -0.2, 0.5);
        assert!(close(
            shift.after(&quarter).apply(p),
            quarter.apply(p) + Vec3::Z
        ));
        assert!(close(quarter.inverse().apply(quarter.apply(p)), p));
        let half = Rigid::IDENTITY.lerp(&quarter, 0.5);
        let eighth = Rigid::about(pivot, Quat::from_rotation_z(std::f32::consts::FRAC_PI_4));
        assert!(close(half.rotation * Vec3::X, eighth.rotation * Vec3::X));
    }

    #[test]
    fn blend_is_rigid_and_keeps_single_motions() {
        let pivot = Vec3::new(0.0, 1.0, 0.0);
        let bent = Rigid::about(pivot, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let shifted = Rigid {
            rotation: Quat::from_rotation_y(0.4),
            translation: Vec3::new(0.2, -0.1, 0.3),
        };
        for g in [bent, shifted, Rigid::IDENTITY] {
            let b = Rigid::blend(&[(g, 0.7)]).unwrap();
            let p = Vec3::new(0.3, -0.2, 0.5);
            assert!(close(b.apply(p), g.apply(p)));
        }
        // Halfway between rest and a quarter turn about the joint: an
        // eighth turn about it, so a point at the joint stays and a point
        // off it keeps its distance (a linear blend would pull it in).
        let half = Rigid::blend(&[(Rigid::IDENTITY, 0.5), (bent, 0.5)]).unwrap();
        assert!(close(half.apply(pivot), pivot));
        let p = pivot + Vec3::X;
        assert!((half.apply(p).distance(pivot) - 1.0).abs() < 1e-5);
        // Opposite signs of the same rotation blend as one.
        let flipped = Rigid {
            rotation: -bent.rotation,
            translation: bent.translation,
        };
        let same = Rigid::blend(&[(bent, 0.5), (flipped, 0.5)]).unwrap();
        assert!(close(same.apply(p), bent.apply(p)));
        assert!(Rigid::blend(&[(bent, 0.0)]).is_none());
    }
}
