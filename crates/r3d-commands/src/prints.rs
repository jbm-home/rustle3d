//! Garments' prints as pixels and texture coordinates (spec 25): the
//! built-ins drawn in the garment's colour, images read from the project
//! folder, and where each cloth vertex falls on the print. Shared by the
//! app, the headless renders and the 3D exports.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use glam::DVec2;
use r3d_core::print::{BuiltinPrint, builtin};
use r3d_core::{GarmentId, PrintSource, Project, Texture};

use crate::drape::{Drape, Piece};

/// Largest side of a print as drawn (pixels); bigger images are halved
/// until they fit. The exports carry the original file.
pub const MAX_DRAWN_SIDE: u32 = 2048;

/// A garment's print, ready to draw.
#[derive(Debug)]
pub struct PrintImage {
    /// Changes whenever the pixels do (another print, colour or file).
    pub key: u64,
    /// The pixels, at most [`MAX_DRAWN_SIDE`] a side.
    pub texture: Arc<Texture>,
    /// Their average colour (sRGB `0xRRGGBB`): the sheen's tint.
    pub mean_rgb: u32,
    /// The image file as the user gave it (PNG or JPEG), or the built-in
    /// as PNG: what the exports embed.
    pub file: Vec<u8>,
    /// `image/png` or `image/jpeg`.
    pub mime: &'static str,
    /// The original image's height over its width.
    pub aspect: f64,
}

/// What a print's pixels depend on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Builtin(BuiltinPrint, u32),
    /// The file, its length and when it was last changed.
    Image(String, u64, Option<std::time::SystemTime>),
}

/// Prints already drawn or read, so an app redrawing every frame does
/// not decode a file or draw a built-in again. A file that cannot be read
/// is remembered too (as nothing) until it changes.
#[derive(Debug, Default)]
pub struct Prints {
    cache: HashMap<Key, Option<Arc<PrintImage>>>,
}

fn hash(key: &Key) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

impl Prints {
    /// The print of `garment`, its image read from `dir` (the project
    /// folder; none: images are left out). `None` for a plain garment or
    /// an image that is missing or unreadable: the garment shows its
    /// colour.
    pub fn of(
        &mut self,
        project: &Project,
        dir: Option<&Path>,
        garment: GarmentId,
    ) -> Option<Arc<PrintImage>> {
        let g = project.garments().get(&garment)?;
        let print = g.print.as_ref()?;
        let key = match &print.source {
            PrintSource::Builtin(b) => Key::Builtin(*b, g.rgb()),
            PrintSource::Image(path) => {
                if !r3d_core::print::image_path_ok(path) {
                    return None;
                }
                let meta = std::fs::metadata(dir?.join(path)).ok()?;
                Key::Image(path.clone(), meta.len(), meta.modified().ok())
            }
        };
        if let Some(found) = self.cache.get(&key) {
            return found.clone();
        }
        let made = match &key {
            Key::Builtin(b, rgb) => {
                let texture = builtin(*b, *rgb);
                Some(PrintImage {
                    key: hash(&key),
                    file: r3d_io::texture::encode_png(&texture),
                    mime: "image/png",
                    aspect: 1.0,
                    mean_rgb: texture.mean_rgb(),
                    texture: Arc::new(texture),
                })
            }
            Key::Image(path, ..) => dir.and_then(|d| {
                let bytes = std::fs::read(d.join(path)).ok()?;
                let texture = r3d_io::texture::decode(&bytes).ok()?;
                let aspect = texture.aspect();
                let texture = texture.fitted(MAX_DRAWN_SIDE);
                Some(PrintImage {
                    key: hash(&key),
                    mime: r3d_io::texture::mime(&bytes)?,
                    aspect,
                    mean_rgb: texture.mean_rgb(),
                    texture: Arc::new(texture),
                    file: bytes,
                })
            }),
        }
        .map(Arc::new);
        // Old versions of an edited file are dropped.
        if let Key::Image(path, ..) = &key {
            self.cache
                .retain(|k, _| !matches!(k, Key::Image(p, ..) if p == path));
        }
        self.cache.insert(key, made.clone());
        made
    }

    /// The print of every garment that has one (and can be read).
    pub fn all(
        &mut self,
        project: &Project,
        dir: Option<&Path>,
    ) -> HashMap<GarmentId, Arc<PrintImage>> {
        project
            .garments()
            .keys()
            .filter_map(|id| Some((*id, self.of(project, dir, *id)?)))
            .collect()
    }
}

/// Texture coordinates of `piece`'s particles on its garment's print
/// (spec 25), in repeats; one unit per metre along the grain without a
/// print. `aspect` is the print image's height over width (1 when
/// unknown).
pub fn piece_uvs(project: &Project, drape: &Drape, piece: &Piece, aspect: f64) -> Vec<[f32; 2]> {
    let Some(frame) = project.print_frame(piece.panel, aspect) else {
        return Vec::new();
    };
    drape
        .scene
        .particles
        .get(piece.particles.clone())
        .unwrap_or_default()
        .iter()
        .map(|p| {
            // Particles rest on the pattern in metres.
            let mm = DVec2::new(f64::from(p.rest.x), f64::from(p.rest.y)) * 1000.0;
            let uv = frame.uv(mm);
            [uv.x as f32, uv.y as f32]
        })
        .collect()
}

/// The aspect of `piece`'s garment's print among `prints` (1 without).
pub fn aspect_of(
    project: &Project,
    prints: &HashMap<GarmentId, Arc<PrintImage>>,
    piece: &Piece,
) -> f64 {
    project
        .panel(piece.panel)
        .and_then(|p| prints.get(&p.garment))
        .map_or(1.0, |p| p.aspect)
}
