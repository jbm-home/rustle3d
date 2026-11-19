#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Offscreen renders of the board and the 3D view. Set `R3D_RENDER_OUT` to a
//! folder to keep the PNGs for visual review.

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_avatar::model::BodyModel;
use r3d_commands::drape;
use r3d_commands::{Script, Session};
use r3d_core::{Fabric, Project};
use r3d_render::board::BoardView;
use r3d_render::offscreen::{PNG_FORMAT, render_png};
use r3d_render::{
    BoardRenderer, BoardScene, Draw, Gpu, Material, Mesh, OrbitCamera, Target, Theme,
    ViewRenderer3d,
};

fn skirt_project() -> Project {
    let script = Script::parse(include_str!("../../../demos/m2/skirt.json")).unwrap();
    let mut s = Session::new(Project::new("t"));
    script.run(|step| s.run_step(step)).unwrap();
    s.project().clone()
}

/// Decode a PNG and count distinct colours (a blank render has one).
fn distinct_colours(png_bytes: &[u8]) -> usize {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let mut set = std::collections::HashSet::new();
    for px in buf[..info.buffer_size()].chunks(4) {
        set.insert([px[0], px[1], px[2]]);
    }
    set.len()
}

fn keep(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("R3D_RENDER_OUT") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn gpu() -> Option<Gpu> {
    // CI machines without any GPU adapter skip rendering tests.
    Gpu::headless().ok()
}

#[test]
fn board_renders_panels_in_both_themes() {
    let Some(gpu) = gpu() else { return };
    let project = skirt_project();
    let first = *project.panels().keys().next().unwrap();
    let scene = BoardScene::from_project(&project, &[first]);
    let (w, h) = (1200, 700);
    let view = BoardView::fit(scene.bounds(), w, h);
    let mut renderer = BoardRenderer::new(&gpu, PNG_FORMAT);
    for (name, theme) in [
        ("board-light.png", Theme::LIGHT),
        ("board-dark.png", Theme::DARK),
    ] {
        let png = render_png(&gpu, w, h, |view_tex| {
            let target = Target {
                view: view_tex,
                width: w,
                height: h,
            };
            renderer.render(&gpu, target, &scene, &view, &theme);
        })
        .unwrap();
        assert!(distinct_colours(&png) > 10, "{name} looks blank");
        keep(name, &png);
    }
    // Picking: the centre of the first panel's bounding box is inside it.
    let p = scene.panels[0]
        .outline
        .iter()
        .fold(r3d_geom::DVec2::ZERO, |a, b| a + *b)
        / scene.panels[0].outline.len() as f64;
    assert_eq!(scene.panel_at(p), Some(first));
}

#[test]
fn view3d_renders_the_avatar_and_the_cloth() {
    let Some(gpu) = gpu() else { return };
    let model = BodyModel::builtin().unwrap();
    let body = r3d_avatar::presets::builtin()[2].body; // Women M
    let fitted = fit(&model, &body);
    let positions = fitted_positions(&model, &fitted);
    let mut avatar = Mesh {
        positions,
        normals: Vec::new(),
        indices: model.triangles().iter().flatten().copied().collect(),
        colors: Vec::new(),
        uvs: Vec::new(),
    };
    avatar.recompute_normals();
    // The skirt template where the simulation starts it, around the body.
    let mut session = Session::new(Project::new("t"));
    let step = serde_json::json!({"type": "template.apply", "template": "a_line_skirt"});
    session
        .run_step(&r3d_commands::Step::from_value(step).unwrap())
        .unwrap();
    let step = serde_json::json!({"type": "garment.set_print",
        "print": {"builtin": "gingham"}, "width_mm": 30});
    session
        .run_step(&r3d_commands::Step::from_value(step).unwrap())
        .unwrap();
    let form = drape::Form::new(avatar.positions.clone(), model.triangles().to_vec());
    let built = drape::build(session.project(), Some(&form), 25.0).unwrap();
    let scene = &built.scene;
    let mut skirt = Mesh {
        positions: scene.particles.iter().map(|p| p.position).collect(),
        normals: Vec::new(),
        indices: scene.triangles.iter().flat_map(|t| t.vertices).collect(),
        colors: Vec::new(),
        uvs: built
            .pieces
            .iter()
            .flat_map(|p| r3d_commands::prints::piece_uvs(session.project(), &built, p, 1.0))
            .collect(),
    };
    skirt.recompute_normals();
    let gingham = r3d_core::print::builtin(r3d_core::BuiltinPrint::Gingham, 0xA03030);
    let camera = OrbitCamera::default();
    let (w, h) = (900, 900);
    let mut renderer = ViewRenderer3d::new(&gpu, PNG_FORMAT);
    for (name, theme) in [
        ("view3d-light.png", Theme::LIGHT),
        ("view3d-dark.png", Theme::DARK),
    ] {
        let draws = [
            Draw {
                mesh: &avatar,
                material: Material::mannequin(&theme),
                print: None,
            },
            Draw {
                mesh: &skirt,
                material: Material::fabric(Fabric::Denim, None).printed(0xD09898),
                print: Some(r3d_render::Printed {
                    key: 7,
                    texture: &gingham,
                }),
            },
        ];
        let png = render_png(&gpu, w, h, |view_tex| {
            let target = Target {
                view: view_tex,
                width: w,
                height: h,
            };
            renderer.render(&gpu, target, &camera, &draws, &theme);
        })
        .unwrap();
        assert!(distinct_colours(&png) > 100, "{name} looks blank");
        keep(name, &png);
    }
}

#[test]
fn board_pixels_and_points_round_trip() {
    let view = r3d_render::board::BoardView {
        center: r3d_geom::DVec2::new(120.0, -40.0),
        mm_per_px: 0.75,
    };
    let p = r3d_geom::DVec2::new(333.0, 81.5);
    let px = view.to_pixels(p, 800, 600);
    assert!(view.to_board(px.x, px.y, 800, 600).distance(p) < 1e-9);
}
