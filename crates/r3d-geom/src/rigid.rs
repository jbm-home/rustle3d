//! Rigid motions in 3D (`f32`, metres): how a bone of a moving body
//! carries the points attached to it.

use glam::{Quat, Vec3};

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
}
