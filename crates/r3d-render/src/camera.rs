//! Orbit camera for the 3D view (metres, Y up).

use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: Vec3,
    /// Horizontal angle around the target (radians).
    pub yaw: f32,
    /// Vertical angle (radians, clamped to avoid flipping).
    pub pitch: f32,
    pub distance: f32,
    /// Field of view along the shorter side of the view (radians).
    pub fov_y: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.75, 0.0),
            yaw: 0.6,
            pitch: 0.12,
            distance: 2.4,
            fov_y: 35f32.to_radians(),
        }
    }
}

impl OrbitCamera {
    pub fn eye(&self) -> Vec3 {
        let pitch = self.pitch.clamp(-1.4, 1.4);
        self.target
            + self.distance
                * Vec3::new(
                    self.yaw.sin() * pitch.cos(),
                    pitch.sin(),
                    self.yaw.cos() * pitch.cos(),
                )
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        // wgpu uses the DirectX clip-space convention (z in 0..1, y up).
        let view = view::look_at_mat4(self.eye(), self.target, Vec3::Y);
        let aspect = aspect.max(1e-3);
        let proj = directx::perspective(self.vertical_fov(aspect), aspect, 0.05, 50.0);
        proj * view
    }

    /// `fov_y` is the field of view along the *shorter* side, so the subject
    /// stays framed in tall, narrow views as well as wide ones.
    fn vertical_fov(&self, aspect: f32) -> f32 {
        if aspect >= 1.0 {
            self.fov_y
        } else {
            2.0 * ((self.fov_y / 2.0).tan() / aspect).atan()
        }
    }

    /// Rotate by a pointer drag (in pixels).
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.4, 1.4);
    }

    /// Zoom by a scroll amount (positive = closer).
    pub fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance * (1.0 - amount * 0.1)).clamp(0.3, 20.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_views_widen_the_vertical_fov() {
        let c = OrbitCamera::default();
        assert_eq!(c.vertical_fov(1.5), c.fov_y);
        // Horizontal FOV of a half-width view equals fov_y.
        let v = c.vertical_fov(0.5);
        let horizontal = 2.0 * ((v / 2.0).tan() * 0.5).atan();
        assert!((horizontal - c.fov_y).abs() < 1e-5);
    }
}
