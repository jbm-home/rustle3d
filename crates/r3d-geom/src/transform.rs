//! Rigid transforms used by pattern tools.

use glam::DVec2;

/// Reflection across the line through `origin` with direction `dir`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mirror {
    origin: DVec2,
    dir: DVec2,
}

impl Mirror {
    /// `None` if `dir` is zero or not finite.
    pub fn new(origin: DVec2, dir: DVec2) -> Option<Self> {
        let dir = dir.try_normalize()?;
        origin.is_finite().then_some(Self { origin, dir })
    }

    pub fn apply(&self, p: DVec2) -> DVec2 {
        let v = p - self.origin;
        self.origin + 2.0 * v.dot(self.dir) * self.dir - v
    }

    /// A displacement reflected (the axis' position does not matter); a
    /// zero one stays exactly zero.
    pub fn apply_vector(&self, v: DVec2) -> DVec2 {
        if v == DVec2::ZERO {
            return DVec2::ZERO;
        }
        2.0 * v.dot(self.dir) * self.dir - v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_axis() {
        let m = Mirror::new(DVec2::new(100.0, 0.0), DVec2::Y).unwrap();
        assert_eq!(m.apply(DVec2::new(60.0, 5.0)), DVec2::new(140.0, 5.0));
        assert_eq!(m.apply(m.apply(DVec2::new(3.0, 4.0))), DVec2::new(3.0, 4.0));
        assert!(Mirror::new(DVec2::ZERO, DVec2::ZERO).is_none());
        assert_eq!(m.apply_vector(DVec2::new(3.0, 4.0)), DVec2::new(-3.0, 4.0));
    }
}
