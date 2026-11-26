#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Path-traced photos. Set `R3D_RENDER_OUT` to a folder to keep the PNGs
//! for visual review. Skipped on machines without any GPU adapter.

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_avatar::model::BodyModel;
use r3d_commands::{Session, drape};
use r3d_core::{Fabric, Project, Texture};
use r3d_render::photo::{
    self, Cloth, Environment, Lighting, PhotoRenderer, PhotoScene, Quality, Settings, Surface,
};
use r3d_render::{Gpu, Mesh, OrbitCamera, Printed, RenderError};

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
    skirt_scene_printed(fabric, None)
}

/// The same, the skirt in `print` (2 cm repeats) if given.
fn skirt_scene_printed(fabric: Fabric, print: Option<&Texture>) -> PhotoScene {
    let model = BodyModel::builtin().unwrap();
    let body = r3d_avatar::presets::builtin()[2].body; // Women M
    let fitted = fit(&model, &body);
    let mut avatar = Mesh {
        positions: fitted_positions(&model, &fitted),
        normals: Vec::new(),
        indices: model.triangles().iter().flatten().copied().collect(),
        colors: Vec::new(),
        uvs: Vec::new(),
    };
    avatar.recompute_normals();
    let mut session = Session::new(Project::new("t"));
    let step = serde_json::json!({"type": "template.apply", "template": "a_line_skirt"});
    session
        .run_step(&r3d_commands::Step::from_value(step).unwrap())
        .unwrap();
    let form = drape::Form::new(avatar.positions.clone(), model.triangles().to_vec());
    if print.is_some() {
        let step = serde_json::json!({"type": "garment.set_print",
            "print": {"builtin": "stripes"}, "width_mm": 20});
        session
            .run_step(&r3d_commands::Step::from_value(step).unwrap())
            .unwrap();
    }
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
                uvs: r3d_commands::prints::piece_uvs(session.project(), &built, p, 1.0),
            };
            m.recompute_normals();
            m
        })
        .collect();
    let surface = Surface::fabric(fabric, None);
    let cloth: Vec<Cloth<'_>> = pieces
        .iter()
        .map(|mesh| Cloth {
            mesh,
            surface,
            print: print.map(|texture| Printed { key: 1, texture }),
        })
        .collect();
    PhotoScene::new(&cloth, &[], Some(&avatar))
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
fn a_striped_skirt_shows_its_stripes() {
    let Some(gpu) = gpu() else { return };
    let stripes = r3d_core::print::builtin(r3d_core::BuiltinPrint::Stripes, 0x1F3A70);
    let camera = OrbitCamera::facing(0.0);
    let settings = Settings::new(Quality::Quick, Lighting::Studio);
    let shoot = |print| {
        let scene = skirt_scene_printed(Fabric::Cotton, print);
        let png = photo::render_png(&gpu, &scene, &camera, settings, 320, 240).unwrap();
        keep(
            if print.is_some() {
                "photo-stripes.png"
            } else {
                "photo-plain.png"
            },
            &png,
        );
        pixels(&png)
    };
    let (w, _, plain) = shoot(None);
    let (_, _, striped) = shoot(Some(&stripes));
    // Blue and off-white stripes side by side: far more change across a row.
    assert!(
        roughness(w, &striped) > 2.0 * roughness(w, &plain),
        "{} vs {}",
        roughness(w, &striped),
        roughness(w, &plain)
    );
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

/// A built-in environment from the application's set.
fn builtin_environment(name: &str) -> Environment {
    let (_, bytes) = r3d_commands::environments::BUILTIN
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap();
    let hdr = r3d_io::hdr::decode(bytes).unwrap();
    Environment::new(hdr.width, hdr.height, |i| {
        r3d_io::hdr::texel_rgb(hdr.rgbe[i])
    })
    .unwrap()
}

#[test]
fn a_photo_in_each_built_in_environment() {
    let Some(gpu) = gpu() else { return };
    let camera = OrbitCamera::default();
    let settings = Settings::new(Quality::Quick, Lighting::Studio);
    for name in r3d_commands::environments::builtin_names() {
        let scene = skirt_scene(Fabric::Denim).with_environment(builtin_environment(name));
        let png = photo::render_png(&gpu, &scene, &camera, settings, 160, 120).unwrap();
        let luma = mean_luma(&pixels(&png).2);
        assert!(luma > 3.0 && luma < 250.0, "{name}: {luma}");
        keep(&format!("photo-env-{name}.png"), &png);
    }
}

#[test]
fn environment_photos_are_deterministic_and_turn_by_whole_turns() {
    let Some(gpu) = gpu() else { return };
    let camera = OrbitCamera::facing(0.3);
    let settings = Settings {
        samples: 4,
        ..Settings::new(Quality::Quick, Lighting::Studio)
    };
    let scene = skirt_scene(Fabric::Silk).with_environment(builtin_environment("ferndale"));
    let shoot = |turn_deg| {
        let settings = Settings {
            turn_deg,
            ..settings
        };
        photo::render_png(&gpu, &scene, &camera, settings, 96, 72).unwrap()
    };
    let a = shoot(30.0);
    assert_eq!(a, shoot(30.0));
    // A whole turn more: the same picture, up to rounding.
    let (_, _, pa) = pixels(&a);
    let (_, _, pb) = pixels(&shoot(390.0));
    let diff: f64 = pa
        .iter()
        .zip(&pb)
        .map(|(x, y)| (f64::from(*x) - f64::from(*y)).abs())
        .sum::<f64>()
        / pa.len() as f64;
    assert!(diff < 1.0, "{diff}");
    // A half turn: another picture.
    let (_, _, pc) = pixels(&shoot(210.0));
    assert_ne!(pa, pc);
}

/// A photo of `scene` refined to the end, as PNG bytes.
fn finish(gpu: &Gpu, photo: &mut PhotoRenderer) -> Vec<u8> {
    while !photo.is_done() {
        photo.sample(gpu, 4);
    }
    photo.resolve(gpu);
    photo.png(gpu).unwrap()
}

#[test]
fn aiming_again_gives_the_photo_taken_from_there() {
    let Some(gpu) = gpu() else { return };
    let settings = Settings {
        samples: 4,
        ..Settings::new(Quality::Quick, Lighting::Studio)
    };
    let from = OrbitCamera::facing(0.3);
    let mut to = OrbitCamera::facing(1.2);
    to.fov_y = 20f32.to_radians();
    let turned = Settings {
        turn_deg: 40.0,
        brightness: 1.5,
        ..settings
    };
    for scene in [
        skirt_scene(Fabric::Cotton),
        skirt_scene(Fabric::Cotton).with_environment(builtin_environment("pav")),
    ] {
        let direct = photo::render_png(&gpu, &scene, &to, turned, 96, 72).unwrap();
        let mut photo = PhotoRenderer::new(&gpu, &scene, &from, settings, 64, 64).unwrap();
        finish(&gpu, &mut photo);
        assert!(photo.aim(&to, 40.0, 1.5));
        photo.resize(&gpu, 96, 72).unwrap();
        assert_eq!(photo.samples_done(), 0);
        assert_eq!(finish(&gpu, &mut photo), direct);
    }
}

#[test]
fn the_camera_stays_inside_the_studio() {
    let Some(gpu) = gpu() else { return };
    let settings = Settings::new(Quality::Quick, Lighting::Studio);
    let near = OrbitCamera::default();
    let far = OrbitCamera {
        distance: 15.0,
        ..near
    };
    let mut photo =
        PhotoRenderer::new(&gpu, &skirt_scene(Fabric::Cotton), &near, settings, 32, 32).unwrap();
    assert!(!photo.aim(&far, 0.0, 1.0));
    // Under an environment there is no backdrop to leave.
    let scene = skirt_scene(Fabric::Cotton).with_environment(builtin_environment("pav"));
    let mut photo = PhotoRenderer::new(&gpu, &scene, &near, settings, 32, 32).unwrap();
    assert!(photo.aim(&far, 0.0, 1.0));
}

#[test]
fn the_floor_catches_shadows_and_shows_the_environment() {
    let Some(gpu) = gpu() else { return };
    // A 2 m square plate 30 cm above the floor, under an even orange sky.
    let plate = Mesh {
        positions: [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(x, z)| glam::Vec3::new(x, 0.3, z))
            .to_vec(),
        normals: Vec::new(),
        indices: vec![0, 2, 1, 0, 3, 2],
        colors: Vec::new(),
        uvs: Vec::new(),
    };
    let surface = Surface::fabric(Fabric::Cotton, Some("#808080"));
    let colour = [1.0, 0.5, 0.25];
    let env = Environment::new(64, 32, |_| colour).unwrap();
    let scene = PhotoScene::new(&[], &[(&plate, surface)], None).with_environment(env);
    // From the side, low: the floor under the plate shows below it.
    let camera = OrbitCamera {
        target: glam::Vec3::new(0.0, 0.15, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        distance: 3.0,
        fov_y: 35f32.to_radians(),
    };
    let settings = Settings {
        samples: 64,
        ..Settings::new(Quality::Quick, Lighting::Studio)
    };
    let (w, h) = (160, 160);
    let png = photo::render_png(&gpu, &scene, &camera, settings, w, h).unwrap();
    keep("photo-env-floor.png", &png);
    let (_, _, px) = pixels(&png);
    let at = |p: glam::Vec3| {
        let (xy, _) = camera.project(p, w, h).unwrap();
        let i = (xy.y as usize * w as usize + xy.x as usize) * 4;
        [px[i], px[i + 1], px[i + 2]].map(f64::from)
    };
    let luma = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let under = at(glam::Vec3::new(0.0, 0.0, 0.0));
    let open = at(glam::Vec3::new(0.0, 0.0, 2.0));
    let sky = at(glam::Vec3::new(0.0, 1.2, 0.0));
    assert!(luma(under) < 0.5 * luma(open), "{under:?} vs {open:?}");
    // In the open the floor is the sky's colour (it shows the
    // environment, lit as it is).
    for k in 0..3 {
        assert!(
            (open[k] - sky[k]).abs() < 0.15 * sky[0],
            "{open:?} vs {sky:?}"
        );
    }
    assert!(sky[0] > sky[1] && sky[1] > sky[2], "{sky:?}");
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
