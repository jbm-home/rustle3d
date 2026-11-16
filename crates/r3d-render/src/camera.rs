//! Orbit camera for the 3D view (metres, Y up).

use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec2, Vec3};

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
            // Frames a standing adult (about 1.8 m) from head to toe.
            target: Vec3::new(0.0, 0.9, 0.0),
            yaw: 0.5,
            pitch: 0.08,
            distance: 3.6,
            fov_y: 35f32.to_radians(),
        }
    }
}

impl OrbitCamera {
    /// The default framing, looking at the body from `yaw` (radians: 0 =
    /// from the front, π/2 = from its left side, π = from the back).
    pub fn facing(yaw: f32) -> Self {
        Self {
            yaw,
            pitch: 0.05,
            ..Self::default()
        }
    }

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

    /// Where `p` appears in a `width × height` view: pixel position (x
    /// right, y down) and depth along the viewing direction (m). `None`
    /// behind the camera.
    pub fn project(&self, p: Vec3, width: u32, height: u32) -> Option<(Vec2, f32)> {
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let clip = self.view_proj(w / h) * p.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let pixel = Vec2::new((ndc.x + 1.0) / 2.0 * w, (1.0 - ndc.y) / 2.0 * h);
        Some((pixel, (p - self.eye()).dot(self.forward())))
    }

    /// The ray through pixel (`x`, `y`): origin (the eye) and unit
    /// direction.
    pub fn ray(&self, x: f32, y: f32, width: u32, height: u32) -> (Vec3, Vec3) {
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let ndc = Vec2::new(2.0 * x / w - 1.0, 1.0 - 2.0 * y / h);
        let inverse = self.view_proj(w / h).inverse();
        let near = inverse.project_point3(ndc.extend(0.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        let eye = self.eye();
        (eye, (far - near).try_normalize().unwrap_or(self.forward()))
    }

    /// Unit viewing direction.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye())
            .try_normalize()
            .unwrap_or(Vec3::NEG_Z)
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
    fn facing_looks_from_that_side() {
        let front = OrbitCamera::facing(0.0).eye();
        assert!(front.z > 3.0 && front.x.abs() < 1e-5);
        let left = OrbitCamera::facing(std::f32::consts::FRAC_PI_2).eye();
        assert!(left.x > 3.0);
    }

    #[test]
    fn rays_go_through_projected_points() {
        let c = OrbitCamera::default();
        let p = Vec3::new(0.1, 1.2, -0.05);
        for (w, h) in [(800, 600), (300, 900)] {
            let (pixel, depth) = c.project(p, w, h).unwrap();
            let (origin, dir) = c.ray(pixel.x, pixel.y, w, h);
            let along = (p - origin).dot(dir);
            assert!((origin + dir * along - p).length() < 1e-4);
            assert!((depth - (p - c.eye()).dot(c.forward())).abs() < 1e-6);
        }
        // The target is in the middle of the view.
        let (centre, _) = c.project(c.target, 800, 600).unwrap();
        assert!((centre - Vec2::new(400.0, 300.0)).length() < 1e-2);
    }

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
