//! Shared wgpu renderers for the 2D pattern board, the 3D view and
//! path-traced photos ([`photo`], see `docs/rendering.md`).
//!
//! The same code draws into the desktop app (a GPU texture shown by the UI)
//! and into offscreen textures saved as PNG for the CLI and MCP, so agents
//! see exactly what users see. Colours come from a [`Theme`], which the UI
//! design system provides. See `docs/architecture.md` §7.2.
//!
//! Shaders write sRGB-encoded colours into non-sRGB targets (`Bgra8Unorm`
//! for the UI texture, `Rgba8Unorm` for PNG), so both look identical.

pub mod blit;
pub mod board;
pub mod camera;
pub mod fitmap;
pub mod gpu;
pub mod mesh;
pub mod offscreen;
pub mod photo;
pub mod print;
pub mod theme;
pub mod view3d;

mod targets;

pub use blit::ImageBlit;
pub use board::{BoardRenderer, BoardScene};
pub use camera::OrbitCamera;
pub use gpu::{Gpu, RenderError};
pub use mesh::Mesh;
pub use print::Printed;
pub use theme::Theme;
pub use view3d::{Draw, Material, ViewRenderer3d};

/// Samples per pixel for anti-aliasing.
pub const MSAA_SAMPLES: u32 = 4;

/// Where a renderer draws: a texture view and its size in pixels.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}
