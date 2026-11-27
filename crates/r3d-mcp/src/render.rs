//! PNG images of the pattern board or the 3D garment, drawn offscreen by
//! `r3d-render` (the same renderer as the app). Used by the `render_board`
//! and `render_3d` tools and by `r3d render`; `render_photo` path-traces
//! the garment in a studio (`r3d_render::photo`).

use glam::Vec3;
use r3d_commands::CommandError;
use r3d_commands::Workspace;
use r3d_commands::drape;
use r3d_commands::fit::FitMap;
use r3d_commands::prints::{PrintImage, Prints, aspect_of, piece_uvs};
use r3d_commands::techpack::{ExportTechPack, Image};
use r3d_core::Fabric;
use r3d_core::{Localizable, Message};
use r3d_render::board::BoardView;
use r3d_render::fitmap;
use r3d_render::offscreen::{PNG_FORMAT, encode_png, render_pixels, render_png};
use r3d_render::photo::{self, Cloth, Lighting, PhotoScene, Quality, Settings, Surface};
use r3d_render::{
    BoardRenderer, BoardScene, Draw, Gpu, Material, Mesh, OrbitCamera, Printed, Target, Theme,
    ViewRenderer3d,
};
use serde_json::{Value, json};
use std::sync::Arc;

/// What to draw.
#[derive(Clone, Debug, PartialEq)]
pub enum View {
    /// The 2D pattern.
    Board,
    /// The garment on the avatar (the cached drape if it is up to date),
    /// coloured by a fit map if one is given.
    Garment(Option<FitMap>),
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

/// Where a photo's camera stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhotoCamera {
    /// From one side, framing the garment (`close_up`) or the whole body.
    Angle { angle: CameraAngle, close_up: bool },
    /// Where the project's photo scene put it.
    Scene(OrbitCamera),
}

/// How to take a photo.
#[derive(Clone, Debug, PartialEq)]
pub struct PhotoOptions {
    /// Samples, bounces, the studio, the light's turn and brightness.
    pub settings: Settings,
    pub camera: PhotoCamera,
    /// The environment that lights the photo instead of the studio (spec
    /// 35): a built-in name or a `.hdr` file in the project folder.
    pub environment: Option<String>,
}

impl Default for PhotoOptions {
    fn default() -> Self {
        Self {
            settings: Settings::new(Quality::Good, Lighting::Studio),
            camera: PhotoCamera::Angle {
                angle: CameraAngle::default(),
                close_up: true,
            },
            environment: None,
        }
    }
}

impl PhotoOptions {
    /// Options from a tool's arguments: `quality`, `lighting`, `camera`,
    /// `frame`, `environment`, `turn_deg` and `brightness`, and the
    /// advanced `samples`, `bounces`, `denoise`. What they leave out comes
    /// from the project's photo `scene` when it has one (spec 35): its
    /// light, turn, brightness and camera.
    /// Unknown keys are left to the caller.
    pub fn from_args(
        args: &serde_json::Map<String, Value>,
        scene: Option<&r3d_core::PhotoSetup>,
    ) -> Result<Self, CommandError> {
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
            None => None,
            Some(l) => Some(
                Lighting::from_name(l)
                    .ok_or_else(|| choice("lighting", &Lighting::ALL.map(Lighting::name)))?,
            ),
        };
        let angle = match text("camera")? {
            None => None,
            Some(a) => Some(
                CameraAngle::from_name(a)
                    .ok_or_else(|| choice("camera", &CameraAngle::ALL.map(CameraAngle::name)))?,
            ),
        };
        let close_up = match text("frame")? {
            None => None,
            Some("garment") => Some(true),
            Some("body") => Some(false),
            Some(_) => return Err(choice("frame", &["garment", "body"])),
        };
        // The scene's light, unless the arguments name one.
        let (lighting, environment) = match (text("environment")?, lighting) {
            (Some(name), _) => (Lighting::Studio, Some(name.to_owned())),
            (None, Some(l)) => (l, None),
            (None, None) => match scene.map(|s| &s.light) {
                Some(r3d_core::PhotoLight::Soft) => (Lighting::Soft, None),
                Some(r3d_core::PhotoLight::Dramatic) => (Lighting::Dramatic, None),
                Some(r3d_core::PhotoLight::Environment { name }) => {
                    (Lighting::Studio, Some(name.clone()))
                }
                Some(r3d_core::PhotoLight::Studio) | None => (Lighting::Studio, None),
            },
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
        let number =
            |key: &str, range: std::ops::RangeInclusive<f32>, default: f32| match args.get(key) {
                None => Ok(default),
                Some(v) => v
                    .as_f64()
                    .map(|n| n as f32)
                    .filter(|n| range.contains(n))
                    .ok_or_else(|| {
                        malformed(
                            key,
                            format!(
                                "expected a number from {} to {}",
                                range.start(),
                                range.end()
                            ),
                        )
                    }),
            };
        settings.turn_deg = number(
            "turn_deg",
            -360_000.0..=360_000.0,
            scene.map_or(0.0, |s| s.turn_deg as f32),
        )?;
        settings.brightness = number(
            "brightness",
            photo::BRIGHTNESS,
            scene.map_or(1.0, |s| s.brightness as f32),
        )?;
        settings.place_size_m = number(
            "place_size_m",
            photo::PLACE_SIZE,
            scene.map_or(photo::PLACE_SIZE_DEFAULT, |s| s.place_size_m as f32),
        )?;
        let camera = match (angle, close_up, scene) {
            (None, None, Some(s)) => PhotoCamera::Scene(OrbitCamera::from_photo(&s.camera)),
            _ => PhotoCamera::Angle {
                angle: angle.unwrap_or_default(),
                close_up: close_up.unwrap_or(true),
            },
        };
        Ok(Self {
            settings,
            camera,
            environment,
        })
    }

    /// Keys [`Self::from_args`] reads.
    pub const KEYS: [&'static str; 11] = [
        "quality",
        "lighting",
        "camera",
        "frame",
        "samples",
        "bounces",
        "denoise",
        "environment",
        "turn_deg",
        "brightness",
        "place_size_m",
    ];
}

/// The size of a photo: the one asked for, else the project's photo
/// scene's, else `default`.
pub fn photo_size(ws: &Workspace, asked: Option<(u32, u32)>, default: (u32, u32)) -> (u32, u32) {
    asked
        .or_else(|| {
            ws.session()
                .project()
                .photo()
                .and_then(|s| s.size)
                .map(|[w, h]| (w, h))
        })
        .unwrap_or(default)
}

/// The environment called `name`, from the built-in set or the project
/// folder.
pub fn environment(ws: &Workspace, name: &str) -> Result<photo::Environment, RenderError> {
    let hdr = ws.environment(name)?;
    photo::Environment::new(hdr.width, hdr.height, |i| {
        hdr.rgbe
            .get(i)
            .map_or([0.0; 3], |t| r3d_commands::environments::texel_rgb(*t))
    })
    .ok_or_else(|| {
        CommandError::Malformed {
            name: "environment".into(),
            detail: "an empty picture".into(),
        }
        .into()
    })
}

// Width of style lines drawn on the garment, and their lift off the
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
    #[error(transparent)]
    Workspace(#[from] r3d_commands::WorkspaceError),
    #[error("cannot render: {0}")]
    Gpu(String),
}

impl Localizable for RenderError {
    fn message(&self) -> Message {
        match self {
            Self::Command(e) => e.message(),
            Self::Workspace(e) => e.message(),
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
        colors: Vec::new(),
        uvs: Vec::new(),
    };
    mesh.recompute_normals();
    mesh
}

/// A piece of the garment as the 3D views show it.
struct DressedPiece {
    mesh: Mesh,
    fabric: Fabric,
    color: Option<String>,
    /// Its garment's print, if it has one (the mesh then has texture
    /// coordinates on it).
    print: Option<Arc<PrintImage>>,
}

impl DressedPiece {
    fn printed(&self) -> Option<Printed<'_>> {
        self.print.as_ref().map(|p| Printed {
            key: p.key,
            texture: &p.texture,
        })
    }

    /// The material of the 3D view, with the print over it.
    fn material(&self) -> Material {
        let material = Material::fabric(self.fabric, self.color.as_deref());
        match &self.print {
            Some(p) => material.printed(p.mean_rgb),
            None => material,
        }
    }

    /// The surface of photos, its colour the print's average.
    fn surface(&self) -> Surface {
        let hex = self.print.as_ref().map(|p| format!("#{:06X}", p.mean_rgb));
        Surface::fabric(self.fabric, hex.as_deref().or(self.color.as_deref()))
    }
}

/// The garments as the 3D views show them.
struct Dressed {
    avatar: Option<Mesh>,
    /// Each piece with its garment's fabric, colour and print.
    pieces: Vec<DressedPiece>,
    /// Style lines on the cloth.
    lines: Vec<Mesh>,
    /// Buttons and zippers on the cloth, with their look.
    trims: Vec<(Mesh, Material)>,
    /// `simulated` (the cached drape) or `start`.
    state: &'static str,
    /// The facts of the fit map the pieces are coloured by, if any.
    fit: Option<Value>,
}

impl Dressed {
    /// How tall the avatar stands (m); 0 without one.
    fn body_height(&self) -> f32 {
        self.avatar
            .as_ref()
            .map_or(0.0, |a| a.positions.iter().map(|p| p.y).fold(0.0, f32::max))
    }

    /// The latest drape, its pieces coloured by `fit_map` if given.
    fn of(ws: &Workspace, fit_map: Option<FitMap>) -> Result<Self, RenderError> {
        let project = ws.session().project();
        let latest = drape::Latest::load(ws.dir(), project)?;
        let positions = &latest.positions;
        // The avatar as it was at the drape's last frame, if it moved.
        let avatar = latest
            .form
            .as_ref()
            .zip(latest.body.clone())
            .map(|(f, surface)| mesh(surface, f.triangles.iter().flatten().copied().collect()));
        let body = latest
            .body
            .as_deref()
            .zip(latest.form.as_ref().map(|f| &f.triangles[..]));
        let values = fit_map.map(|m| (m, m.values(&latest.drape, positions, body)));
        let prints = Prints::default().all(project, Some(ws.dir()));
        let at = drape::ClothAt::new(&latest.drape, positions);
        let pieces: Vec<DressedPiece> = latest
            .drape
            .pieces
            .iter()
            .filter_map(|p| {
                let garment = project.garment_of(p.panel);
                let mut m = mesh(
                    at.positions.get(p.particles.clone())?.to_vec(),
                    p.indices.clone(),
                );
                if let Some((map, v)) = &values {
                    m.colors = v
                        .get(p.particles.clone())?
                        .iter()
                        .map(|x| fitmap::color(map.level(*x)))
                        .collect();
                }
                let print = project
                    .panel(p.panel)
                    .and_then(|panel| prints.get(&panel.garment))
                    .cloned();
                if print.is_some() {
                    m.uvs = piece_uvs(project, &latest.drape, p, aspect_of(project, &prints, p));
                }
                Some(DressedPiece {
                    mesh: m,
                    fabric: garment.map(|g| g.fabric).unwrap_or_default(),
                    color: garment.and_then(|g| g.color.clone()),
                    print,
                })
            })
            .collect();
        let marks = drape::ClothMarks::new(&latest.drape, project);
        let lines = marks
            .internal_lines(&at)
            .iter()
            .map(|l| Mesh::ribbon(l, LINE_WIDTH_M, LINE_LIFT_M))
            .collect();
        let trims = r3d_render::trims::meshes(
            &marks.buttons(&at),
            &marks.zippers(&at),
            &marks.stitches(&at),
        );
        let fit = values.map(|(map, v)| {
            // The tightest value on the garment, and its piece.
            let (loose, tight) = map.range();
            let tightest = v
                .iter()
                .enumerate()
                .max_by(|a, b| map.level(*a.1).total_cmp(&map.level(*b.1)))
                .map(|(i, x)| {
                    let piece = latest
                        .drape
                        .pieces
                        .iter()
                        .find(|p| p.particles.contains(&i))
                        .and_then(|p| project.panel(p.panel))
                        .map(|p| p.name.clone());
                    json!({ "value": (f64::from(*x) * 10.0).round() / 10.0, "piece": piece })
                });
            json!({
                "map": map.name(),
                "unit": map.unit(),
                "loose": loose,
                "tight": tight,
                "tightest": tightest,
                "legend": "bar at the right edge: tight (yellow) at the top, loose (dark) at the bottom",
            })
        });
        Ok(Self {
            avatar,
            pieces,
            lines,
            trims,
            state: latest.state(),
            fit,
        })
    }
}

/// A camera on the garment from `yaw`, framing it (or the body when
/// there is no cloth).
fn garment_camera(dressed: &Dressed, yaw: f32) -> OrbitCamera {
    let mut points = dressed.pieces.iter().flat_map(|p| p.mesh.positions.iter());
    let garment = points
        .next()
        .map(|first| points.fold((*first, *first), |(lo, hi), p| (lo.min(*p), hi.max(*p))));
    match garment {
        Some((lo, hi)) => OrbitCamera::framing(lo, hi, yaw),
        None => OrbitCamera::facing(yaw),
    }
}

/// A camera from `yaw` on the whole body, head to toe; without one (an
/// object, spec 36), on the cloth.
fn standing_camera(dressed: &Dressed, yaw: f32) -> OrbitCamera {
    if dressed.avatar.is_some() {
        OrbitCamera::facing(yaw).for_height(dressed.body_height())
    } else {
        garment_camera(dressed, yaw)
    }
}

/// The 3D view of the garment (as `render_3d` draws it) through `camera`.
fn garment_png(
    gpu: &Gpu,
    dressed: &Dressed,
    camera: &OrbitCamera,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
    let theme = Theme::LIGHT;
    let mut draws: Vec<Draw<'_>> = Vec::with_capacity(dressed.pieces.len() + 1);
    if let Some(a) = &dressed.avatar {
        draws.push(Draw {
            mesh: a,
            material: Material::mannequin(&theme),
            print: None,
        });
    }
    draws.extend(dressed.pieces.iter().map(|p| {
        match p.mesh.colors.is_empty() {
            true => Draw {
                mesh: &p.mesh,
                material: p.material(),
                print: p.printed(),
            },
            // A fit map: its colours, no sheen nor print over them.
            false => Draw {
                mesh: &p.mesh,
                material: Material {
                    sheen_amount: 0.0,
                    ..Material::fabric(p.fabric, p.color.as_deref())
                },
                print: None,
            },
        }
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
    let mut pixels = render_pixels(gpu, width, height, |view| {
        let target = Target {
            view,
            width,
            height,
        };
        renderer.render(gpu, target, camera, &draws, &theme);
    })
    .map_err(gpu_failure)?;
    if dressed.fit.is_some() {
        fitmap::paint_legend(&mut pixels, width, height);
    }
    encode_png(width, height, &pixels).map_err(gpu_failure)
}

/// Which pictures of the garment open a tech pack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TechPackViews {
    /// The 3D view, front and back (fast).
    #[default]
    View3d,
    /// Quick photos, front and back (seconds to a minute).
    Photo,
    /// None.
    None,
}

impl TechPackViews {
    pub const NAMES: [&'static str; 3] = ["3d", "photo", "none"];

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "3d" => Some(Self::View3d),
            "photo" => Some(Self::Photo),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// Size of each tech pack picture (pixels, portrait).
const TECHPACK_VIEW: (u32, u32) = (900, 1200);

/// The front and back pictures of the garment for a tech pack, captioned
/// with `request`'s labels: the cached drape if it is up to date,
/// otherwise where the simulation starts.
pub fn techpack_views(
    gpu: &Gpu,
    ws: &Workspace,
    views: TechPackViews,
    request: &ExportTechPack,
) -> Result<Vec<Image>, RenderError> {
    let (w, h) = TECHPACK_VIEW;
    let sides = [
        (CameraAngle::Front, request.label("front")),
        (CameraAngle::Back, request.label("back")),
    ];
    let pngs: Vec<(Vec<u8>, &str)> = match views {
        TechPackViews::None => Vec::new(),
        TechPackViews::View3d => {
            let dressed = Dressed::of(ws, None)?;
            sides
                .iter()
                .map(|(angle, caption)| {
                    let camera = garment_camera(&dressed, angle.yaw());
                    Ok((garment_png(gpu, &dressed, &camera, w, h)?, *caption))
                })
                .collect::<Result<_, RenderError>>()?
        }
        TechPackViews::Photo => sides
            .iter()
            .map(|(angle, caption)| {
                let options = PhotoOptions {
                    settings: Settings::new(Quality::Quick, Lighting::Studio),
                    camera: PhotoCamera::Angle {
                        angle: *angle,
                        close_up: true,
                    },
                    environment: None,
                };
                Ok((render(gpu, ws, View::Photo(options), w, h)?.0, *caption))
            })
            .collect::<Result<_, RenderError>>()?,
    };
    pngs.into_iter()
        .map(|(png, caption)| Image::from_png(&png, caption).map_err(gpu_failure))
        .collect()
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
            let prints = Prints::default()
                .all(project, Some(ws.dir()))
                .into_iter()
                .map(|(garment, p)| r3d_render::board::BoardPrint {
                    garment,
                    key: p.key,
                    texture: p.texture.clone(),
                    aspect: p.aspect,
                })
                .collect();
            let scene = BoardScene::from_project(project, &[])
                .with_sizes(project)
                .with_prints(project, prints);
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
        View::Garment(fit_map) => {
            let dressed = Dressed::of(ws, fit_map)?;
            let camera = standing_camera(&dressed, OrbitCamera::default().yaw);
            let png = garment_png(gpu, &dressed, &camera, width, height)?;
            let mut facts = json!({ "view": "3d", "drape": dressed.state, "pieces": dressed.pieces.len(), "avatar": dressed.avatar.is_some() });
            if let Some(fit) = &dressed.fit {
                facts["fit_map"] = fit.clone();
            }
            Ok((png, facts))
        }
        View::Photo(options) => {
            let start = std::time::Instant::now();
            let dressed = Dressed::of(ws, None)?;
            let cloth: Vec<Cloth<'_>> = dressed
                .pieces
                .iter()
                .map(|p| Cloth {
                    mesh: &p.mesh,
                    surface: p.surface(),
                    print: p.printed(),
                })
                .collect();
            let extras: Vec<(&Mesh, Surface)> = dressed
                .trims
                .iter()
                .map(|(mesh, material)| (mesh, Surface::plain(material.base)))
                .collect();
            let mut scene = PhotoScene::new(&cloth, &extras, dressed.avatar.as_ref());
            if let Some(name) = &options.environment {
                scene = scene.with_environment(environment(ws, name)?);
            }
            let camera = match options.camera {
                PhotoCamera::Angle {
                    angle,
                    close_up: true,
                } => garment_camera(&dressed, angle.yaw()),
                PhotoCamera::Angle { angle, .. } => standing_camera(&dressed, angle.yaw()),
                PhotoCamera::Scene(camera) => camera,
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
            let mut facts = json!({
                "view": "photo",
                "drape": dressed.state,
                "pieces": dressed.pieces.len(),
                "avatar": dressed.avatar.is_some(),
                "samples": s.samples,
                "bounces": s.bounces,
                "denoised": s.denoise,
                "seconds": (start.elapsed().as_secs_f64() * 10.0).round() / 10.0,
            });
            match &options.environment {
                Some(name) => facts["environment"] = json!(name),
                None => facts["lighting"] = json!(s.lighting.name()),
            }
            facts["size"] = json!([width, height]);
            facts["turn_deg"] = json!(s.turn_deg);
            facts["brightness"] = json!(s.brightness);
            if options.environment.is_some() {
                facts["place_size_m"] = json!(s.place_size_m);
            }
            facts["camera"] = json!(match options.camera {
                PhotoCamera::Angle { angle, .. } => angle.name(),
                PhotoCamera::Scene(_) => "scene",
            });
            Ok((png, facts))
        }
    }
}
