//! PNG images of the pattern board or the 3D garment, drawn offscreen by
//! `r3d-render` (the same renderer as the app). Used by the `render_board`
//! and `render_3d` tools and by `r3d render`; `render_photo` path-traces
//! the garment in a studio (`r3d_render::photo`).

use glam::Vec3;
use r3d_commands::CommandError;
use r3d_commands::Workspace;
use r3d_commands::drape;
use r3d_core::Fabric;
use r3d_core::{Localizable, Message};
use r3d_render::board::BoardView;
use r3d_render::offscreen::{PNG_FORMAT, render_png};
use r3d_render::photo::{self, Lighting, PhotoScene, Quality, Settings, Surface};
use r3d_render::{
    BoardRenderer, BoardScene, Draw, Gpu, Material, Mesh, OrbitCamera, Target, Theme,
    ViewRenderer3d,
};
use serde_json::{Value, json};

/// What to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    /// The 2D pattern.
    Board,
    /// The garment on the avatar (the cached drape if it is up to date).
    Garment,
    /// A path-traced photo of the garment on the avatar.
    Photo(PhotoOptions),
}

/// Where the photo's camera stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraAngle {
    Front,
    #[default]
    ThreeQuarter,
    Side,
    Back,
}

impl CameraAngle {
    pub const ALL: [CameraAngle; 4] = [
        CameraAngle::Front,
        CameraAngle::ThreeQuarter,
        CameraAngle::Side,
        CameraAngle::Back,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            CameraAngle::Front => "front",
            CameraAngle::ThreeQuarter => "three_quarter",
            CameraAngle::Side => "side",
            CameraAngle::Back => "back",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Angle around the body (radians, 0 = from the front).
    fn yaw(self) -> f32 {
        match self {
            CameraAngle::Front => 0.0,
            CameraAngle::ThreeQuarter => 0.5,
            CameraAngle::Side => std::f32::consts::FRAC_PI_2,
            CameraAngle::Back => std::f32::consts::PI,
        }
    }
}

/// How to take a photo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoOptions {
    pub settings: Settings,
    pub angle: CameraAngle,
    /// Frame the garment (true) or the whole body.
    pub close_up: bool,
}

impl Default for PhotoOptions {
    fn default() -> Self {
        Self {
            settings: Settings::new(Quality::Good, Lighting::Studio),
            angle: CameraAngle::default(),
            close_up: true,
        }
    }
}

impl PhotoOptions {
    /// Options from a tool's arguments: `quality`, `lighting`, `camera`,
    /// `frame`, and the advanced `samples`, `bounces`, `denoise`.
    /// Unknown keys are left to the caller.
    pub fn from_args(args: &serde_json::Map<String, Value>) -> Result<Self, CommandError> {
        let malformed = |name: &str, detail: String| CommandError::Malformed {
            name: name.into(),
            detail,
        };
        let text = |key: &str| -> Result<Option<&str>, CommandError> {
            match args.get(key) {
                None => Ok(None),
                Some(v) => v
                    .as_str()
                    .map(Some)
                    .ok_or_else(|| malformed(key, "expected a string".into())),
            }
        };
        let choice = |key: &str, names: &[&str]| {
            malformed(key, format!("expected one of: {}", names.join(", ")))
        };
        let quality = match text("quality")? {
            None => Quality::Good,
            Some(q) => Quality::from_name(q)
                .ok_or_else(|| choice("quality", &Quality::ALL.map(Quality::name)))?,
        };
        let lighting = match text("lighting")? {
            None => Lighting::Studio,
            Some(l) => Lighting::from_name(l)
                .ok_or_else(|| choice("lighting", &Lighting::ALL.map(Lighting::name)))?,
        };
        let angle = match text("camera")? {
            None => CameraAngle::default(),
            Some(a) => CameraAngle::from_name(a)
                .ok_or_else(|| choice("camera", &CameraAngle::ALL.map(CameraAngle::name)))?,
        };
        let close_up = match text("frame")? {
            None | Some("garment") => true,
            Some("body") => false,
            Some(_) => return Err(choice("frame", &["garment", "body"])),
        };
        let mut settings = Settings::new(quality, lighting);
        let count = |key: &str, max: u32| -> Result<Option<u32>, CommandError> {
            match args.get(key) {
                None => Ok(None),
                Some(v) => v
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .filter(|n| (1..=max).contains(n) || (key == "bounces" && *n == 0))
                    .map(Some)
                    .ok_or_else(|| malformed(key, format!("expected a whole number up to {max}"))),
            }
        };
        if let Some(n) = count("samples", photo::MAX_SAMPLES)? {
            settings.samples = n;
        }
        if let Some(n) = count("bounces", photo::MAX_BOUNCES)? {
            settings.bounces = n;
        }
        if let Some(v) = args.get("denoise") {
            settings.denoise = v
                .as_bool()
                .ok_or_else(|| malformed("denoise", "expected true or false".into()))?;
        }
        Ok(Self {
            settings,
            angle,
            close_up,
        })
    }

    /// Keys [`Self::from_args`] reads.
    pub const KEYS: [&'static str; 7] = [
        "quality", "lighting", "camera", "frame", "samples", "bounces", "denoise",
    ];
}

/// Width of style lines drawn on the garment, and their lift off the
/// cloth (m), as in the app.
const LINE_WIDTH_M: f32 = 0.003;
const LINE_LIFT_M: f32 = 0.0015;

/// Smallest and largest image side (pixels).
pub const SIZE_RANGE: std::ops::RangeInclusive<u32> = 16..=4096;

/// Why a render failed.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error("cannot render: {0}")]
    Gpu(String),
}

impl Localizable for RenderError {
    fn message(&self) -> Message {
        match self {
            Self::Command(e) => e.message(),
            Self::Gpu(detail) => Message::new("gpu", self.to_string()).arg("detail", detail),
        }
    }
}

fn gpu_failure(e: impl std::fmt::Display) -> RenderError {
    RenderError::Gpu(e.to_string())
}

/// A GPU for offscreen renders (no window).
pub fn headless_gpu() -> Result<Gpu, RenderError> {
    Gpu::headless().map_err(gpu_failure)
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

/// The garments as the 3D views show them.
struct Dressed {
    avatar: Option<Mesh>,
    /// Each piece with its garment's fabric and colour.
    pieces: Vec<(Mesh, Fabric, Option<String>)>,
    /// Style lines on the cloth.
    lines: Vec<Mesh>,
    /// `simulated` (the cached drape) or `start`.
    state: &'static str,
}

impl Dressed {
    fn of(ws: &Workspace) -> Result<Self, RenderError> {
        let project = ws.session().project();
        let form = drape::Form::of_project(project)?;
        let built = drape::prepare(project, form.as_ref())?;
        let start: Vec<Vec3> = built.scene.particles.iter().map(|p| p.position).collect();
        let cached = drape::load_cache(ws.dir(), project, start.len());
        let state = if cached.is_some() {
            "simulated"
        } else {
            "start"
        };
        // The avatar as it was at the drape's last frame, if it moved.
        let avatar = form.map(|f| {
            let surface = match &cached {
                Some(c) => f.posed(c.animation.as_ref(), c.time),
                None => f.positions.clone(),
            };
            mesh(surface, f.triangles.iter().flatten().copied().collect())
        });
        let positions = cached.map_or(start, |c| c.positions);
        let pieces = built
            .pieces
            .iter()
            .filter_map(|p| {
                let garment = project.garment_of(p.panel);
                Some((
                    mesh(
                        positions.get(p.particles.clone())?.to_vec(),
                        p.indices.clone(),
                    ),
                    garment.map(|g| g.fabric).unwrap_or_default(),
                    garment.and_then(|g| g.color.clone()),
                ))
            })
            .collect();
        let lines = drape::LinesOnCloth::new(&built, &positions)
            .internal_lines(project)
            .iter()
            .map(|l| Mesh::ribbon(l, LINE_WIDTH_M, LINE_LIFT_M))
            .collect();
        Ok(Self {
            avatar,
            pieces,
            lines,
            state,
        })
    }
}

/// Render `view` as PNG bytes of `width × height`, plus facts about what
/// was drawn.
pub fn render(
    gpu: &Gpu,
    ws: &Workspace,
    view: View,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, Value), RenderError> {
    if !SIZE_RANGE.contains(&width) || !SIZE_RANGE.contains(&height) {
        let (lo, hi) = (*SIZE_RANGE.start(), *SIZE_RANGE.end());
        let range = r3d_commands::Invalid::Range(lo.into(), hi.into());
        return Err(CommandError::invalid("size", range).into());
    }
    let theme = Theme::LIGHT;
    match view {
        View::Board => {
            let project = ws.session().project();
            let scene = BoardScene::from_project(project, &[]).with_sizes(project);
            let board_view = BoardView::fit(scene.bounds(), width, height);
            let mut renderer = BoardRenderer::new(gpu, PNG_FORMAT);
            let png = render_png(gpu, width, height, |view| {
                let target = Target {
                    view,
                    width,
                    height,
                };
                renderer.render(gpu, target, &scene, &board_view, &theme);
            })
            .map_err(gpu_failure)?;
            Ok((
                png,
                json!({ "view": "board", "panels": scene.panels.len() }),
            ))
        }
        View::Garment => {
            let dressed = Dressed::of(ws)?;
            let mut draws: Vec<Draw<'_>> = Vec::with_capacity(dressed.pieces.len() + 1);
            if let Some(a) = &dressed.avatar {
                draws.push(Draw {
                    mesh: a,
                    material: Material::mannequin(&theme),
                });
            }
            draws.extend(dressed.pieces.iter().map(|(m, fabric, color)| Draw {
                mesh: m,
                material: Material::fabric(*fabric, color.as_deref()),
            }));
            draws.extend(dressed.lines.iter().map(|mesh| Draw {
                mesh,
                material: Material {
                    base: theme.panel_line,
                    sheen: theme.panel_line,
                    sheen_amount: 0.0,
                },
            }));
            let camera = OrbitCamera::default();
            let mut renderer = ViewRenderer3d::new(gpu, PNG_FORMAT);
            let png = render_png(gpu, width, height, |view| {
                let target = Target {
                    view,
                    width,
                    height,
                };
                renderer.render(gpu, target, &camera, &draws, &theme);
            })
            .map_err(gpu_failure)?;
            Ok((
                png,
                json!({ "view": "3d", "drape": dressed.state, "pieces": dressed.pieces.len(), "avatar": dressed.avatar.is_some() }),
            ))
        }
        View::Photo(options) => {
            let start = std::time::Instant::now();
            let dressed = Dressed::of(ws)?;
            let cloth: Vec<(&Mesh, Surface)> = dressed
                .pieces
                .iter()
                .map(|(m, fabric, color)| (m, Surface::fabric(*fabric, color.as_deref())))
                .collect();
            let scene = PhotoScene::new(&cloth, dressed.avatar.as_ref());
            let mut points = dressed
                .pieces
                .iter()
                .flat_map(|(m, _, _)| m.positions.iter());
            let garment = points
                .next()
                .map(|first| points.fold((*first, *first), |(lo, hi), p| (lo.min(*p), hi.max(*p))));
            let camera = match garment {
                Some((lo, hi)) if options.close_up => {
                    OrbitCamera::framing(lo, hi, options.angle.yaw())
                }
                _ => OrbitCamera::facing(options.angle.yaw()),
            };
            let png = photo::render_png(gpu, &scene, &camera, options.settings, width, height)
                .map_err(|e| match e {
                    r3d_render::RenderError::TooLarge { .. } => {
                        let most = r3d_render::photo::MAX_PIXELS;
                        CommandError::invalid(
                            "pixels",
                            r3d_commands::Invalid::Range(1.0, most as f64),
                        )
                        .into()
                    }
                    other => gpu_failure(other),
                })?;
            let s = options.settings;
            Ok((
                png,
                json!({
                    "view": "photo",
                    "drape": dressed.state,
                    "pieces": dressed.pieces.len(),
                    "avatar": dressed.avatar.is_some(),
                    "samples": s.samples,
                    "bounces": s.bounces,
                    "lighting": s.lighting.name(),
                    "denoised": s.denoise,
                    "seconds": (start.elapsed().as_secs_f64() * 10.0).round() / 10.0,
                }),
            ))
        }
    }
}
