//! `r3d render`: PNG images of the pattern board or the 3D garment, drawn
//! offscreen by `r3d-render` (the same renderer as the app).

use glam::Vec3;
use r3d_commands::Workspace;
use r3d_commands::drape;
use r3d_commands::queries::body_model;
use r3d_core::Message;
use r3d_render::board::BoardView;
use r3d_render::offscreen::{PNG_FORMAT, render_png};
use r3d_render::{
    BoardRenderer, BoardScene, Draw, Gpu, Material, Mesh, OrbitCamera, Target, Theme,
    ViewRenderer3d,
};
use serde_json::{Value, json};

use crate::Failure;

/// What to draw.
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum View {
    /// The 2D pattern.
    Board,
    /// The garment on the avatar (the cached drape if it is up to date).
    #[value(name = "3d")]
    Garment,
}

fn gpu_failure(e: impl std::fmt::Display) -> Failure {
    Failure {
        message: Message::new("gpu", format!("cannot render: {e}")),
        code: 2,
    }
}

fn mesh(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    let mut mesh = Mesh {
        positions,
        normals: Vec::new(),
        indices,
    };
    mesh.recompute_normals();
    mesh
}

/// Render `view` as PNG bytes of `width × height`, plus facts about what
/// was drawn.
pub fn render(
    ws: &Workspace,
    view: View,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, Value), Failure> {
    let gpu = Gpu::headless().map_err(gpu_failure)?;
    let project = ws.session().project();
    let theme = Theme::LIGHT;
    match view {
        View::Board => {
            let scene = BoardScene::from_project(project, &[]);
            let board_view = BoardView::fit(scene.bounds(), width, height);
            let mut renderer = BoardRenderer::new(&gpu, PNG_FORMAT);
            let png = render_png(&gpu, width, height, |view| {
                let target = Target {
                    view,
                    width,
                    height,
                };
                renderer.render(&gpu, target, &scene, &board_view, &theme);
            })
            .map_err(gpu_failure)?;
            Ok((
                png,
                json!({ "view": "board", "panels": scene.panels.len() }),
            ))
        }
        View::Garment => {
            let body = drape::fitted_body(project)?;
            let built = drape::prepare(project, body.as_deref())?;
            let start: Vec<Vec3> = built.scene.particles.iter().map(|p| p.position).collect();
            let cached = drape::load_cache(ws.dir(), project, start.len());
            let drape_state = if cached.is_some() {
                "simulated"
            } else {
                "start"
            };
            let positions = cached.unwrap_or(start);
            let garment = project
                .default_garment()
                .and_then(|id| project.garments().get(&id));
            let fabric = Material::fabric(
                garment.map(|g| g.fabric).unwrap_or_default(),
                garment.and_then(|g| g.color.as_deref()),
            );
            let avatar = match body {
                Some(b) => Some(mesh(
                    b,
                    body_model()?
                        .triangles()
                        .iter()
                        .flatten()
                        .copied()
                        .collect(),
                )),
                None => None,
            };
            let pieces: Vec<Mesh> = built
                .pieces
                .iter()
                .filter_map(|p| {
                    Some(mesh(
                        positions.get(p.particles.clone())?.to_vec(),
                        p.indices.clone(),
                    ))
                })
                .collect();
            let mut draws: Vec<Draw<'_>> = Vec::with_capacity(pieces.len() + 1);
            if let Some(a) = &avatar {
                draws.push(Draw {
                    mesh: a,
                    material: Material::mannequin(&theme),
                });
            }
            draws.extend(pieces.iter().map(|m| Draw {
                mesh: m,
                material: fabric,
            }));
            let camera = OrbitCamera::default();
            let mut renderer = ViewRenderer3d::new(&gpu, PNG_FORMAT);
            let png = render_png(&gpu, width, height, |view| {
                let target = Target {
                    view,
                    width,
                    height,
                };
                renderer.render(&gpu, target, &camera, &draws, &theme);
            })
            .map_err(gpu_failure)?;
            Ok((
                png,
                json!({ "view": "3d", "drape": drape_state, "pieces": pieces.len(), "avatar": avatar.is_some() }),
            ))
        }
    }
}
