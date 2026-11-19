//! The tech pack's pictures in the app: the garment as the 3D view shows it
//! now, from the front and the back (`docs/specs/23-production.md`).

use r3d_commands::techpack::{ExportTechPack, Image};
use r3d_render::offscreen::{PNG_FORMAT, render_png};
use r3d_render::{Draw, Gpu, Material, Mesh, OrbitCamera, Target, Theme, ViewRenderer3d};

use crate::cloth::ClothPiece;

/// Size of each picture (pixels, portrait).
const VIEW: (u32, u32) = (900, 1200);

/// What the 3D view shows: the cloth with its materials, the lines drawn
/// on it and the body.
#[derive(Debug, Default)]
pub(crate) struct Dressed {
    pub cloth: Vec<(ClothPiece, Material)>,
    pub lines: Vec<Mesh>,
    /// Buttons and zippers on the cloth.
    pub trims: Vec<(Mesh, Material)>,
    pub body: Option<Mesh>,
}

/// The front and the back of `dressed`, framed on the cloth and captioned
/// with `request`'s labels; none without cloth.
pub(crate) fn views(
    gpu: &Gpu,
    dressed: &Dressed,
    request: &ExportTechPack,
) -> Result<Vec<Image>, String> {
    let mut points = dressed
        .cloth
        .iter()
        .flat_map(|(p, _)| p.mesh.positions.iter());
    let Some(first) = points.next() else {
        return Ok(Vec::new());
    };
    let (lo, hi) = points.fold((*first, *first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let theme = Theme::LIGHT;
    let mut draws: Vec<Draw<'_>> = Vec::new();
    if let Some(body) = &dressed.body {
        draws.push(Draw {
            mesh: body,
            material: Material::mannequin(&theme),
            print: None,
        });
    }
    draws.extend(dressed.cloth.iter().map(|(piece, material)| Draw {
        mesh: &piece.mesh,
        material: *material,
        print: piece.printed(),
    }));
    draws.extend(dressed.lines.iter().map(|mesh| Draw {
        mesh,
        material: Material {
            base: theme.panel_line,
            sheen: theme.panel_line,
            sheen_amount: 0.0,
        },
        print: None,
    }));
    draws.extend(dressed.trims.iter().map(|(mesh, material)| Draw {
        mesh,
        material: *material,
        print: None,
    }));
    let mut renderer = ViewRenderer3d::new(gpu, PNG_FORMAT);
    let (width, height) = VIEW;
    [
        (0.0, request.label("front")),
        (std::f32::consts::PI, request.label("back")),
    ]
    .into_iter()
    .map(|(yaw, caption)| {
        let camera = OrbitCamera::framing(lo, hi, yaw);
        let png = render_png(gpu, width, height, |view| {
            let target = Target {
                view,
                width,
                height,
            };
            renderer.render(gpu, target, &camera, &draws, &theme);
        })
        .map_err(|e| e.to_string())?;
        Image::from_png(&png, caption).map_err(|e| e.to_string())
    })
    .collect()
}
