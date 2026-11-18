#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Path-traced photos. Set `R3D_RENDER_OUT` to a folder to keep the PNGs
//! for visual review. Skipped on machines without any GPU adapter.

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_avatar::model::BodyModel;
use r3d_commands::{Session, drape};
use r3d_core::{Fabric, Project};
use r3d_render::photo::{self, Lighting, PhotoRenderer, PhotoScene, Quality, Settings, Surface};
use r3d_render::{Gpu, Mesh, OrbitCamera, RenderError};

fn gpu() -> Option<Gpu> {
    Gpu::headless().ok()
}

fn keep(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("R3D_RENDER_OUT") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

/// RGBA pixels of a PNG.
fn pixels(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn mean_luma(px: &[u8]) -> f64 {
    let sum: f64 = px
        .chunks(4)
        .map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2]))
        .sum();
    sum / (px.len() / 4) as f64
}

/// Mean absolute difference between horizontal neighbours (noise).
fn roughness(w: u32, px: &[u8]) -> f64 {
    let mut sum = 0.0;
    let mut n = 0;
    for row in px.chunks(w as usize * 4) {
        for pair in row.chunks(4).collect::<Vec<_>>().windows(2) {
            sum += (f64::from(pair[0][1]) - f64::from(pair[1][1])).abs();
            n += 1;
        }
    }
    sum / f64::from(n)
}

/// The avatar in an A-line skirt where the simulation starts it.
fn skirt_scene(fabric: Fabric) -> PhotoScene {
    let model = BodyModel::builtin().unwrap();
    let body = r3d_avatar::presets::builtin()[2].body; // Women M
    let fitted = fit(&model, &body);
    let mut avatar = Mesh {
        positions: fitted_positions(&model, &fitted),
        normals: Vec::new(),
        indices: model.triangles().iter().flatten().copied().collect(),
        colors: Vec::new(),
    };
    avatar.recompute_normals();
    let mut session = Session::new(Project::new("t"));
    let step = serde_json::json!({"type": "template.apply", "template": "a_line_skirt"});
    session
        .run_step(&r3d_commands::Step::from_value(step).unwrap())
        .unwrap();
    let form = drape::Form::new(avatar.positions.clone(), model.triangles().to_vec());
    let built = drape::build(session.project(), Some(&form), 25.0).unwrap();
    let pieces: Vec<Mesh> = built
        .pieces
        .iter()
        .map(|p| {
            let mut m = Mesh {
                positions: built.scene.particles[p.particles.clone()]
                    .iter()
                    .map(|q| q.position)
                    .collect(),
                normals: Vec::new(),
                indices: p.indices.clone(),
                colors: Vec::new(),
            };
            m.recompute_normals();
            m
        })
        .collect();
    let surface = Surface::fabric(fabric, None);
    let cloth: Vec<(&Mesh, Surface)> = pieces.iter().map(|m| (m, surface)).collect();
    PhotoScene::new(&cloth, Some(&avatar))
}

#[test]
fn a_photo_of_the_garment_in_each_lighting() {
    let Some(gpu) = gpu() else { return };
    let scene = skirt_scene(Fabric::Denim);
    assert!(scene.triangle_count() > 10_000);
    let camera = OrbitCamera::default();
    for lighting in Lighting::ALL {
        let settings = Settings::new(Quality::Quick, lighting);
        let png = photo::render_png(&gpu, &scene, &camera, settings, 240, 180).unwrap();
        let (w, h, px) = pixels(&png);
        assert_eq!((w, h), (240, 180));
        let luma = mean_luma(&px);
        assert!(luma > 3.0 && luma < 250.0, "{lighting:?}: {luma}");
        keep(&format!("photo-{}.png", lighting.name()), &png);
    }
}

#[test]
fn the_same_photo_twice_is_the_same() {
    let Some(gpu) = gpu() else { return };
    let scene = skirt_scene(Fabric::Silk);
    let camera = OrbitCamera::facing(0.3);
    let settings = Settings {
        samples: 4,
        ..Settings::new(Quality::Quick, Lighting::Studio)
    };
    let a = photo::render_png(&gpu, &scene, &camera, settings, 96, 72).unwrap();
    let b = photo::render_png(&gpu, &scene, &camera, settings, 96, 72).unwrap();
    assert_eq!(a, b);
}

#[test]
fn more_samples_and_the_denoiser_make_a_smoother_image_of_the_same_brightness() {
    let Some(gpu) = gpu() else { return };
    let scene = skirt_scene(Fabric::Cotton);
    let camera = OrbitCamera::default();
    let base = Settings::new(Quality::Quick, Lighting::Studio);
    let render = |samples, denoise| {
        let settings = Settings {
            samples,
            denoise,
            ..base
        };
        let png = photo::render_png(&gpu, &scene, &camera, settings, 160, 120).unwrap();
        pixels(&png)
    };
    let (w, _, few) = render(2, false);
    let (_, _, many) = render(32, false);
    let (_, _, filtered) = render(2, true);
    keep_rgba("photo-2spp.png", w, &few);
    keep_rgba("photo-2spp-denoised.png", w, &filtered);
    // Same light on average (within the noise of 2 samples).
    let (lf, lm) = (mean_luma(&few), mean_luma(&many));
    assert!((lf - lm).abs() < 0.05 * lm, "{lf} vs {lm}");
    assert!(roughness(w, &many) < roughness(w, &few));
    assert!(roughness(w, &filtered) < roughness(w, &few) * 0.6);
}

fn keep_rgba(name: &str, w: u32, px: &[u8]) {
    let h = (px.len() / 4) as u32 / w;
    keep(name, &r3d_render::offscreen::encode_png(w, h, px).unwrap());
}

#[test]
fn progressive_rendering_counts_its_samples() {
    let Some(gpu) = gpu() else { return };
    let scene = PhotoScene::default();
    let settings = Settings {
        samples: 5,
        ..Settings::new(Quality::Quick, Lighting::Soft)
    };
    let mut photo =
        PhotoRenderer::new(&gpu, &scene, &OrbitCamera::default(), settings, 32, 24).unwrap();
    assert_eq!(photo.samples_done(), 0);
    photo.sample(&gpu, 3);
    assert_eq!(photo.samples_done(), 3);
    assert!(!photo.is_done());
    photo.sample(&gpu, 10);
    assert!(photo.is_done() && photo.samples_done() == 5);
    photo.resolve(&gpu);
    let (w, h, _) = pixels(&photo.png(&gpu).unwrap());
    assert_eq!((w, h), (32, 24));
}

#[test]
fn oversized_photos_are_refused() {
    let Some(gpu) = gpu() else { return };
    let settings = Settings::new(Quality::Quick, Lighting::Studio);
    let scene = PhotoScene::default();
    for (w, h) in [(0, 10), (4097, 10), (4096, 4096)] {
        let r = PhotoRenderer::new(&gpu, &scene, &OrbitCamera::default(), settings, w, h);
        assert!(matches!(r, Err(RenderError::TooLarge { .. })), "{w}x{h}");
    }
    assert!(
        PhotoRenderer::new(&gpu, &scene, &OrbitCamera::default(), settings, 3840, 2160).is_ok()
    );
}
