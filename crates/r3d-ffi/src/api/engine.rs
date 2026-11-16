//! The engine a UI window talks to: one project folder, plus the views the
//! UI displays (the 2D board and the 3D garment), rendered by Rust into
//! textures the UI composites without copying.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use flutter_rust_bridge::frb;

use r3d_commands::drape::{self, Drape, Form};
use r3d_commands::{Script, Step, Workspace};
use r3d_sim::Solver;

use crate::cloth::{
    Adjust, CORNER_PICK_PX, ClothHit, EDGE_PICK_MM, EDGE_PICK_PX, Held, Hit, PICK_PX, RESYNC_EVERY,
    SEAM_END, STROKE_STEP_MM, Sync, Tool, ray_triangle,
};
use crate::sim::SimThread;
use r3d_core::{Body, EdgeId, Fabric, Localizable, Message, PanelId};
use r3d_render::board::BoardView;
use r3d_render::theme::PIECE_COLORS;
use r3d_render::{
    BoardRenderer, BoardScene, Draw, Gpu, Material, Mesh, OrbitCamera, Target, Theme,
    ViewRenderer3d,
};

/// A structured, translatable error (see `docs/architecture.md` §7.7).
#[derive(Debug, Clone)]
pub struct ApiError {
    /// Stable code, the translation key.
    pub code: String,
    /// Named arguments for the translation.
    pub args: HashMap<String, String>,
    /// English fallback.
    pub text: String,
}

impl From<Message> for ApiError {
    fn from(m: Message) -> Self {
        Self {
            code: m.code.to_owned(),
            args: m.args.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            text: m.text,
        }
    }
}

impl ApiError {
    pub(crate) fn new(code: &str, text: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            args: HashMap::new(),
            text: text.into(),
        }
    }
}

pub(crate) fn err(e: impl Localizable) -> ApiError {
    e.message().into()
}

/// Canvas colours as `0xAARRGGBB` (Flutter's `Color.toARGB32()`).
#[derive(Debug, Clone)]
pub struct CanvasColors {
    pub board_background: u32,
    pub grid_minor: u32,
    pub grid_major: u32,
    pub panel_line: u32,
    pub selection: u32,
    pub sky_top: u32,
    pub sky_bottom: u32,
    pub ground: u32,
    pub avatar: u32,
    /// Piece fills, cycled; exactly `PIECE_COLORS` (8) entries.
    pub pieces: Vec<u32>,
}

fn rgba(argb: u32) -> [f32; 4] {
    let c = |shift: u32| ((argb >> shift) & 0xff) as f32 / 255.0;
    [c(16), c(8), c(0), c(24)]
}

impl CanvasColors {
    pub(crate) fn to_theme(&self) -> Result<Theme, ApiError> {
        let pieces: [u32; PIECE_COLORS] = self.pieces.clone().try_into().map_err(|_| {
            ApiError::new(
                "invalid.piece_colors",
                format!("expected {PIECE_COLORS} piece colours"),
            )
        })?;
        Ok(Theme {
            board_background: rgba(self.board_background),
            grid_minor: rgba(self.grid_minor),
            grid_major: rgba(self.grid_major),
            pieces: pieces.map(rgba),
            panel_line: rgba(self.panel_line),
            selection: rgba(self.selection),
            sky_top: rgba(self.sky_top),
            sky_bottom: rgba(self.sky_bottom),
            ground: rgba(self.ground),
            avatar: rgba(self.avatar),
        })
    }
}

/// What a press on the garment does when dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// Pull the cloth.
    Pull,
    /// On a hem: lengthen; on a seam: take in or let out.
    Adjust,
    /// Move the piece around the body (its placement).
    Move,
}

/// Where the 3D view looks at the body from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraSide {
    Front,
    ThreeQuarter,
    /// The body's left side.
    Left,
    Back,
    Right,
}

/// What a view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    /// The 2D pattern board.
    Board,
    /// The 3D garment.
    Garment,
}

struct View {
    kind: ViewKind,
    texture: wgpu::Texture,
    target: wgpu::TextureView,
    width: u32,
    height: u32,
    /// Board pan/zoom chosen by the user; `None` fits the pattern.
    board_view: Option<BoardView>,
}

/// GPU state, created when the first view is attached.
struct Graphics {
    gpu: Gpu,
    board: BoardRenderer,
    garment: ViewRenderer3d,
    views: HashMap<u32, View>,
    next_view: u32,
    camera: OrbitCamera,
    theme: Theme,
    /// The fitted avatar mesh, for the body it was fitted to.
    avatar: Option<(Body, Mesh)>,
}

impl Graphics {
    fn new() -> Result<Self, ApiError> {
        let gpu = Gpu::headless().map_err(|e| ApiError::new("gpu", e.to_string()))?;
        let format = wgpu::TextureFormat::Bgra8Unorm;
        Ok(Self {
            board: BoardRenderer::new(&gpu, format),
            garment: ViewRenderer3d::new(&gpu, format),
            gpu,
            views: HashMap::new(),
            next_view: 1,
            camera: OrbitCamera::default(),
            theme: Theme::LIGHT,
            avatar: None,
        })
    }
}

/// A fabric choice for the UI.
#[derive(Debug, Clone)]
pub struct FabricSwatch {
    /// The value `garment.set_fabric` takes (`cotton`, `denim`…).
    pub id: String,
    /// Default colour, ARGB.
    pub argb: u32,
}

/// A pattern change made by dragging in 3D, for the label by the cursor.
#[derive(Debug, Clone)]
pub struct PatternChange {
    /// `length` (hem moved; positive = longer) or `fit` (seam taken in;
    /// positive = taken in, negative = let out).
    pub kind: String,
    pub mm: f64,
}

/// State of the cloth simulation, for the UI.
#[derive(Debug, Clone)]
pub struct SimStatus {
    /// A garment is loaded (placed around the body).
    pub loaded: bool,
    pub running: bool,
    /// Simulated time (s).
    pub time_s: f64,
    /// Wall time of the last solver step (ms).
    pub step_ms: f64,
    pub particles: u32,
    /// Largest opening of a seam (mm).
    pub max_seam_gap_mm: f64,
}

/// A project opened by the UI.
#[frb(opaque)]
pub struct Engine {
    ws: Mutex<Workspace>,
    graphics: Mutex<Option<Graphics>>,
    /// Selection shared by every view (not saved, not undoable).
    selection: Mutex<Vec<PanelId>>,
    /// The edge picked within the selection, if any.
    selected_edge: Mutex<Option<EdgeId>>,
    /// The simulation thread, started on first use.
    sim: Mutex<Option<SimThread>>,
    /// The loaded simulation's scene (pieces, pattern positions).
    cloth: Mutex<Option<Drape>>,
    /// The dress form for the current measurements.
    form: Mutex<Option<(Body, Form)>>,
    /// Edits not yet carried into the running simulation.
    sync: Mutex<Sync>,
    /// The cloth under the pointer since it went down, if any.
    held: Mutex<Option<Held>>,
    /// A line being drawn on the garment: its panel and pattern points (mm).
    stroke: Mutex<Option<(PanelId, Vec<r3d_geom::DVec2>)>>,
}

fn poisoned() -> ApiError {
    ApiError::new("internal", "engine lock poisoned")
}

/// Width of the pattern lines drawn on the garment, and how far they sit
/// off the cloth (m).
const LINE_WIDTH_M: f32 = 0.003;
const EDGE_WIDTH_M: f32 = 0.006;
const LINE_LIFT_M: f32 = 0.0015;

impl Engine {
    /// Open the project folder at `path`, creating it if it does not exist.
    #[frb(sync)]
    pub fn open(path: String) -> Result<Engine, ApiError> {
        let dir = Path::new(&path);
        let ws = if dir.exists() {
            Workspace::open(dir).map_err(err)?
        } else {
            let name = dir
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into());
            Workspace::create(dir, &name).map_err(err)?
        };
        Ok(Engine {
            ws: Mutex::new(ws),
            graphics: Mutex::new(None),
            selection: Mutex::new(Vec::new()),
            selected_edge: Mutex::new(None),
            sim: Mutex::new(None),
            cloth: Mutex::new(None),
            form: Mutex::new(None),
            sync: Mutex::new(Sync::default()),
            held: Mutex::new(None),
            stroke: Mutex::new(None),
        })
    }

    fn workspace(&self) -> Result<MutexGuard<'_, Workspace>, ApiError> {
        self.ws.lock().map_err(|_| poisoned())
    }

    fn graphics(&self) -> Result<MutexGuard<'_, Option<Graphics>>, ApiError> {
        self.graphics.lock().map_err(|_| poisoned())
    }

    fn selected(&self) -> Result<MutexGuard<'_, Vec<PanelId>>, ApiError> {
        self.selection.lock().map_err(|_| poisoned())
    }

    /// The simulation thread, started if needed.
    fn sim(&self) -> Result<MutexGuard<'_, Option<SimThread>>, ApiError> {
        let mut slot = self.sim.lock().map_err(|_| poisoned())?;
        if slot.is_none() {
            *slot = Some(SimThread::spawn().map_err(|e| ApiError::new("internal", e.to_string()))?);
        }
        Ok(slot)
    }

    /// The dress form for the project's measurements, built when they
    /// change (`None` without an avatar).
    fn form(
        &self,
        project: &r3d_core::Project,
    ) -> Result<MutexGuard<'_, Option<(Body, Form)>>, ApiError> {
        let mut slot = self.form.lock().map_err(|_| poisoned())?;
        let body = project.avatar().map(|a| a.body);
        if slot.as_ref().map(|(b, _)| *b) != body {
            *slot = match (body, Form::of_project(project).map_err(err)?) {
                (Some(b), Some(f)) => Some((b, f)),
                _ => None,
            };
        }
        Ok(slot)
    }

    /// Rebuild the simulation from the document. With `carry`, the cloth
    /// continues from where it is (pattern edits while draping); without,
    /// it starts again around the body, paused.
    fn rebuild(&self, carry: bool) -> Result<(), ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        let form = self.form(project)?;
        let mut drape = drape::prepare(project, form.as_ref().map(|(_, f)| f)).map_err(err)?;
        drop(form);
        drop(ws);
        let sim = self.sim()?;
        let thread = sim.as_ref().ok_or_else(poisoned)?;
        let now = thread.latest();
        let mut cloth = self.cloth.lock().map_err(|_| poisoned())?;
        let carry = carry && now.loaded;
        if carry && let Some(old) = cloth.as_ref() {
            drape::carry_over(old, &now.positions, &mut drape);
        }
        if drape.pieces.is_empty() {
            thread.clear();
        } else {
            let mut solver = Solver::new(drape.scene.clone());
            if carry {
                solver.continue_at(now.time);
            }
            thread.load(solver);
            if carry && now.running {
                thread.set_running(true);
            }
        }
        *cloth = Some(drape);
        Ok(())
    }

    /// Carry pending edits into the running simulation, at most every
    /// `RESYNC_EVERY` (a drag in 2D sends edits every frame).
    fn resync_if_due(&self) -> Result<(), ApiError> {
        let due = {
            let mut sync = self.sync.lock().map_err(|_| poisoned())?;
            let due = sync.pending && sync.last.elapsed() >= RESYNC_EVERY;
            if due {
                sync.pending = false;
                sync.last = Instant::now();
            }
            due
        };
        let loaded = self.cloth.lock().map_err(|_| poisoned())?.is_some();
        if due && loaded {
            self.rebuild(true)?;
        }
        Ok(())
    }

    /// Note that the document changed: the drape follows on a next frame.
    fn edited(&self) -> Result<(), ApiError> {
        self.sync.lock().map_err(|_| poisoned())?.pending = true;
        Ok(())
    }

    /// Start the simulation again from the pattern: the pieces around the
    /// body, paused.
    #[frb(sync)]
    pub fn reset_simulation(&self) -> Result<SimStatus, ApiError> {
        self.rebuild(false)?;
        self.simulation_status()
    }

    /// Run or pause the simulation. Running with nothing loaded loads the
    /// garment first.
    #[frb(sync)]
    pub fn run_simulation(&self, running: bool) -> Result<SimStatus, ApiError> {
        if running && !self.simulation_status()?.loaded {
            self.reset_simulation()?;
        }
        if let Some(thread) = self.sim()?.as_ref() {
            thread.set_running(running);
        }
        self.simulation_status()
    }

    /// The latest simulation state.
    #[frb(sync)]
    pub fn simulation_status(&self) -> Result<SimStatus, ApiError> {
        let p = self
            .sim()?
            .as_ref()
            .map(SimThread::latest)
            .unwrap_or_default();
        Ok(SimStatus {
            loaded: p.loaded,
            running: p.running,
            time_s: p.time.into(),
            step_ms: p.step_ms,
            particles: p.positions.len() as u32,
            max_seam_gap_mm: f64::from(p.max_seam_gap) * 1000.0,
        })
    }

    /// Run one step (a JSON object, as in scripts and MCP) and return its
    /// JSON result. Changes are saved immediately.
    #[frb(sync)]
    pub fn run(&self, step_json: String) -> Result<String, ApiError> {
        let value: serde_json::Value = serde_json::from_str(&step_json).map_err(|e| ApiError {
            code: "malformed".into(),
            args: HashMap::from([("detail".into(), e.to_string())]),
            text: format!("malformed step: {e}"),
        })?;
        let step = Step::from_value(value).map_err(err)?;
        let mut ws = self.workspace()?;
        let result = ws.run_step(&step).map_err(err)?;
        if !matches!(step, Step::Query(_)) {
            self.edited()?;
        }
        if ws.session().is_dirty() {
            ws.save().map_err(err)?;
        }
        // Forget selected panels that no longer exist (e.g. after undo).
        let project = ws.session().project();
        self.selected()?.retain(|id| project.panel(*id).is_some());
        let mut edge = self.selected_edge.lock().map_err(|_| poisoned())?;
        if edge.is_some_and(|e| project.edge(e).is_none()) {
            *edge = None;
        }
        Ok(result.to_string())
    }

    /// Run a script (a JSON array of steps, with `"$N.path"` references to
    /// earlier results) and return the JSON array of results. Used for
    /// templates. The project is saved once at the end.
    #[frb(sync)]
    pub fn run_script(&self, script_json: String) -> Result<String, ApiError> {
        let script = Script::parse(&script_json).map_err(err)?;
        let mut ws = self.workspace()?;
        let results = script.run(|step| ws.run_step(step)).map_err(err)?;
        self.edited()?;
        if ws.session().is_dirty() {
            ws.save().map_err(err)?;
        }
        Ok(serde_json::Value::Array(results).to_string())
    }

    /// The fabrics the UI offers, with the colour each one renders with by
    /// default (ARGB), so swatches match the 3D view.
    #[frb(sync)]
    pub fn fabrics() -> Vec<FabricSwatch> {
        [
            Fabric::Cotton,
            Fabric::Denim,
            Fabric::Jersey,
            Fabric::Silk,
            Fabric::Wool,
        ]
        .into_iter()
        .map(|f| {
            let [r, g, b, _] = Material::fabric(f, None).base;
            let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
            FabricSwatch {
                id: serde_json::to_value(f)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                argb: 0xFF00_0000 | byte(r) << 16 | byte(g) << 8 | byte(b),
            }
        })
        .collect()
    }

    /// Attach a view rendering into a UI texture (`surface`: platform handle
    /// from the UI side, an `IOSurfaceRef` pointer on macOS), or without
    /// one into a texture of its own, read back with `read_view`. Returns
    /// the view id.
    #[frb(sync)]
    pub fn attach_view(
        &self,
        kind: ViewKind,
        surface: Option<u64>,
        width: u32,
        height: u32,
    ) -> Result<u32, ApiError> {
        let mut slot = self.graphics()?;
        if slot.is_none() {
            *slot = Some(Graphics::new()?);
        }
        let g = slot.as_mut().ok_or_else(poisoned)?;
        let texture = match surface {
            Some(surface) => crate::surface::wrap_ui_surface(&g.gpu.device, surface, width, height)
                .map_err(|e| ApiError::new("ui_surface", e))?,
            None => g.gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("view pixels"),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }),
        };
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id = g.next_view;
        g.next_view += 1;
        g.views.insert(
            id,
            View {
                kind,
                texture,
                target,
                width,
                height,
                board_view: None,
            },
        );
        Ok(id)
    }

    /// The last frame of a view attached without a UI surface, as BGRA
    /// pixels, row after row (platforms without shared textures).
    pub fn read_view(&self, view: u32) -> Result<Vec<u8>, ApiError> {
        let slot = self.graphics()?;
        let g = slot
            .as_ref()
            .ok_or_else(|| ApiError::new("no_view", "no view attached"))?;
        let v = g
            .views
            .get(&view)
            .ok_or_else(|| ApiError::new("no_view", format!("unknown view {view}")))?;
        r3d_render::offscreen::read_pixels(&g.gpu, &v.texture)
            .map_err(|e| ApiError::new("gpu", e.to_string()))
    }

    /// Stop rendering into a view (its UI texture is about to go away).
    #[frb(sync)]
    pub fn detach_view(&self, view: u32) -> Result<(), ApiError> {
        if let Some(g) = self.graphics()?.as_mut() {
            g.views.remove(&view);
        }
        Ok(())
    }

    /// Set the canvas colours from the UI design system, so the board and
    /// the 3D view match the app theme.
    #[frb(sync)]
    pub fn set_canvas(&self, colors: CanvasColors) -> Result<(), ApiError> {
        let theme = colors.to_theme()?;
        let mut slot = self.graphics()?;
        if slot.is_none() {
            *slot = Some(Graphics::new()?);
        }
        if let Some(g) = slot.as_mut() {
            g.theme = theme;
        }
        Ok(())
    }

    /// Render a view and wait until the GPU is done, so the UI never shows
    /// a half-drawn frame. Returns the CPU + GPU time in milliseconds.
    #[frb(sync)]
    pub fn render_view(&self, view: u32) -> Result<f64, ApiError> {
        let start = Instant::now();
        self.resync_if_due()?;
        let selected = self.selected()?.clone();
        let ws = self.workspace()?;
        let project = ws.session().project();
        let mut slot = self.graphics()?;
        let g = slot
            .as_mut()
            .ok_or_else(|| ApiError::new("no_view", "no view attached"))?;
        let theme = g.theme;
        let v = g
            .views
            .get(&view)
            .ok_or_else(|| ApiError::new("no_view", format!("unknown view {view}")))?;
        let target = Target {
            view: &v.target,
            width: v.width,
            height: v.height,
        };
        match v.kind {
            ViewKind::Board => {
                let edge = *self.selected_edge.lock().map_err(|_| poisoned())?;
                let scene = BoardScene::from_project(project, &selected).with_selected_edge(edge);
                let board_view = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                g.board.render(&g.gpu, target, &scene, &board_view, &theme);
            }
            ViewKind::Garment => {
                let body = project.avatar().map(|a| a.body);
                if body != g.avatar.as_ref().map(|(b, _)| *b) {
                    let form = self.form(project)?;
                    g.avatar = form.as_ref().map(|(b, f)| {
                        let mut mesh = Mesh {
                            positions: f.positions.clone(),
                            normals: Vec::new(),
                            indices: f.triangles.iter().flatten().copied().collect(),
                        };
                        mesh.recompute_normals();
                        (*b, mesh)
                    });
                }
                let garment = project
                    .default_garment()
                    .and_then(|id| project.garments().get(&id));
                let fabric = Material::fabric(
                    garment.map(|g| g.fabric).unwrap_or_default(),
                    garment.and_then(|g| g.color.as_deref()),
                );
                let published = self
                    .sim
                    .lock()
                    .map_err(|_| poisoned())?
                    .as_ref()
                    .map(SimThread::latest)
                    .unwrap_or_default();
                let cloth = self.cloth.lock().map_err(|_| poisoned())?;
                let edge = *self.selected_edge.lock().map_err(|_| poisoned())?;
                let (lines, edge_line) = cloth
                    .as_ref()
                    .map(|d| {
                        let on = drape::LinesOnCloth::new(d, &published.positions);
                        let lines: Vec<Mesh> = on
                            .internal_lines(project)
                            .iter()
                            .map(|l| Mesh::ribbon(l, LINE_WIDTH_M, LINE_LIFT_M))
                            .collect();
                        let edge = edge
                            .and_then(|e| on.edge(e))
                            .map(|l| Mesh::ribbon(&l, EDGE_WIDTH_M, LINE_LIFT_M));
                        (lines, edge)
                    })
                    .unwrap_or_default();
                let pieces: Vec<(PanelId, Mesh)> = cloth
                    .iter()
                    .flat_map(|d| &d.pieces)
                    .filter_map(|piece| {
                        let positions = published.positions.get(piece.particles.clone())?;
                        let mut mesh = Mesh {
                            positions: positions.to_vec(),
                            normals: Vec::new(),
                            indices: piece.indices.clone(),
                        };
                        mesh.recompute_normals();
                        Some((piece.panel, mesh))
                    })
                    .collect();
                drop(cloth);
                let mut draws = Vec::with_capacity(pieces.len() + 1);
                if let Some((_, avatar)) = &g.avatar {
                    draws.push(Draw {
                        mesh: avatar,
                        material: Material::mannequin(&theme),
                    });
                }
                for (panel, mesh) in &pieces {
                    let material = if selected.contains(panel) {
                        fabric.tinted(theme.selection, 0.45)
                    } else {
                        fabric
                    };
                    draws.push(Draw { mesh, material });
                }
                let ink = |base: [f32; 4]| Material {
                    base,
                    sheen: base,
                    sheen_amount: 0.0,
                };
                draws.extend(lines.iter().map(|mesh| Draw {
                    mesh,
                    material: ink(theme.panel_line),
                }));
                draws.extend(edge_line.iter().map(|mesh| Draw {
                    mesh,
                    material: ink(theme.selection),
                }));
                g.garment.render(&g.gpu, target, &g.camera, &draws, &theme);
            }
        }
        g.gpu
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| ApiError::new("gpu", e.to_string()))?;
        Ok(start.elapsed().as_secs_f64() * 1000.0)
    }

    /// A click at pixel (`x`, `y`) of a view. Updates the shared selection
    /// and returns it (panel ids).
    #[frb(sync)]
    pub fn click(&self, view: u32, x: f64, y: f64) -> Result<Vec<String>, ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        let slot = self.graphics()?;
        let v = slot
            .as_ref()
            .and_then(|g| g.views.get(&view))
            .ok_or_else(|| ApiError::new("no_view", format!("unknown view {view}")))?;
        // The piece under the cursor, and its edge when close to one.
        let (panel, edge) = match v.kind {
            ViewKind::Board => {
                let scene = BoardScene::from_project(project, &[]);
                let board_view = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                let p = board_view.to_board(x, y, v.width, v.height);
                match scene.edge_at(p, EDGE_PICK_PX * board_view.mm_per_px) {
                    Some(e) => (Some(e.panel), Some(e.id)),
                    None => (scene.panel_at(p), None),
                }
            }
            ViewKind::Garment => match self.cloth_hit(v, &slot, x, y)? {
                Some(hit) => (
                    Some(hit.panel),
                    project.edge_near(hit.panel, hit.at, EDGE_PICK_MM),
                ),
                None => (None, None),
            },
        };
        let mut selected = self.selected()?;
        *selected = panel.into_iter().collect();
        *self.selected_edge.lock().map_err(|_| poisoned())? = edge;
        Ok(selected.iter().map(ToString::to_string).collect())
    }

    /// Start drawing on the garment at (`x`, `y`) of a 3D view. Returns
    /// whether the pointer is on the cloth.
    #[frb(sync)]
    pub fn stroke_start(&self, view: u32, x: f64, y: f64) -> Result<bool, ApiError> {
        let slot = self.graphics()?;
        let Some(v) = slot.as_ref().and_then(|g| g.views.get(&view)) else {
            return Ok(false);
        };
        let hit = self.cloth_hit(v, &slot, x, y)?;
        let started = hit.is_some();
        *self.stroke.lock().map_err(|_| poisoned())? = hit.map(|h| (h.panel, vec![h.at]));
        Ok(started)
    }

    /// Continue the line to (`x`, `y`): points on other pieces, or off the
    /// cloth, are skipped.
    #[frb(sync)]
    pub fn stroke_to(&self, view: u32, x: f64, y: f64) -> Result<(), ApiError> {
        let slot = self.graphics()?;
        let Some(v) = slot.as_ref().and_then(|g| g.views.get(&view)) else {
            return Ok(());
        };
        let hit = self.cloth_hit(v, &slot, x, y)?;
        drop(slot);
        if let Some(hit) = hit {
            self.extend_stroke(hit)?;
        }
        Ok(())
    }

    fn extend_stroke(&self, hit: ClothHit) -> Result<(), ApiError> {
        let mut stroke = self.stroke.lock().map_err(|_| poisoned())?;
        if let Some((panel, points)) = stroke.as_mut()
            && hit.panel == *panel
            && points
                .last()
                .is_none_or(|p| p.distance(hit.at) >= STROKE_STEP_MM)
        {
            points.push(hit.at);
        }
        Ok(())
    }

    /// Finish the line: it becomes an internal line on the pattern
    /// (`internal_line.add`; a style line, a pocket placement…), on the
    /// source piece when drawn on a linked mirror copy. Returns the new
    /// line's id, or `None` when the stroke was too short.
    #[frb(sync)]
    pub fn stroke_end(&self) -> Result<Option<String>, ApiError> {
        let Some((panel, mut points)) = self.stroke.lock().map_err(|_| poisoned())?.take() else {
            return Ok(None);
        };
        // On a linked mirror copy, the line goes on its source (the copy
        // follows): copy = reflect(source) + offset.
        let link = self
            .workspace()?
            .session()
            .project()
            .panel(panel)
            .and_then(|p| p.mirror_of.clone());
        let panel = match link {
            Some(link) => {
                if let Some(m) = r3d_geom::Mirror::new(link.origin, link.dir) {
                    for p in &mut points {
                        *p = m.apply(*p - link.offset);
                    }
                }
                link.source
            }
            None => panel,
        };
        let (Some(start), true) = (points.first(), points.len() >= 2) else {
            return Ok(None);
        };
        let segments: Vec<_> = points[1..]
            .iter()
            .map(|p| serde_json::json!({ "kind": "line", "to": [p.x, p.y] }))
            .collect();
        let step = serde_json::json!({
            "type": "internal_line.add", "panel": panel,
            "start": [start.x, start.y], "segments": segments,
        });
        let out: serde_json::Value = serde_json::from_str(&self.run(step.to_string())?)
            .map_err(|e| ApiError::new("internal", e.to_string()))?;
        Ok(out["created"][0].as_str().map(str::to_owned))
    }

    /// The edge picked within the selection (by a click near a border, in
    /// either view), if any.
    #[frb(sync)]
    pub fn selected_edge(&self) -> Result<Option<String>, ApiError> {
        Ok(self
            .selected_edge
            .lock()
            .map_err(|_| poisoned())?
            .map(|e| e.to_string()))
    }

    /// Where the ray through pixel (`x`, `y`) of a 3D view first meets the
    /// cloth: the panel and the point on its pattern (mm), interpolated in
    /// the hit triangle (Möller & Trumbore 1997).
    fn cloth_hit(
        &self,
        v: &View,
        graphics: &Option<Graphics>,
        x: f64,
        y: f64,
    ) -> Result<Option<ClothHit>, ApiError> {
        let Some(g) = graphics.as_ref() else {
            return Ok(None);
        };
        let positions = self
            .sim
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .map(|t| t.latest().positions)
            .unwrap_or_default();
        let (origin, dir) = g.camera.ray(x as f32, y as f32, v.width, v.height);
        let cloth = self.cloth.lock().map_err(|_| poisoned())?;
        let Some(drape) = cloth.as_ref() else {
            return Ok(None);
        };
        let mut best: Option<(f32, ClothHit)> = None;
        for piece in &drape.pieces {
            for t in piece.indices.as_chunks::<3>().0 {
                let idx = t.map(|i| piece.particles.start + i as usize);
                let (Some(a), Some(b), Some(c)) = (
                    positions.get(idx[0]),
                    positions.get(idx[1]),
                    positions.get(idx[2]),
                ) else {
                    continue;
                };
                let Some((dist, u, w)) = ray_triangle(origin, dir, *a, *b, *c) else {
                    continue;
                };
                if best.as_ref().is_some_and(|(d, _)| *d <= dist) {
                    continue;
                }
                let rest = |i: usize| {
                    drape
                        .scene
                        .particles
                        .get(i)
                        .map(|p| p.rest)
                        .unwrap_or_default()
                };
                let at = rest(idx[0]) * (1.0 - u - w) + rest(idx[1]) * u + rest(idx[2]) * w;
                best = Some((
                    dist,
                    ClothHit {
                        panel: piece.panel,
                        at: r3d_geom::DVec2::new(f64::from(at.x), f64::from(at.y)) * 1000.0,
                    },
                ));
            }
        }
        Ok(best.map(|(_, hit)| hit))
    }

    /// The cloth particle under pixel (`x`, `y`) of a 3D view: the front
    /// one within `PICK_PX` pixels.
    fn pick_cloth(
        &self,
        v: &View,
        graphics: &Option<Graphics>,
        x: f64,
        y: f64,
        only: &dyn Fn(usize) -> bool,
    ) -> Result<Option<Hit>, ApiError> {
        let Some(g) = graphics.as_ref() else {
            return Ok(None);
        };
        let positions = self
            .sim
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .map(|t| t.latest().positions)
            .unwrap_or_default();
        let cursor = glam::Vec2::new(x as f32, y as f32);
        let best = positions
            .iter()
            .enumerate()
            .filter(|(i, _)| only(*i))
            .filter_map(|(i, p)| {
                let (pixel, depth) = g.camera.project(*p, v.width, v.height)?;
                (pixel.distance(cursor) < PICK_PX).then_some((i, *p, depth))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let Some((particle, position, _)) = best else {
            return Ok(None);
        };
        let panel = self
            .cloth
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .flat_map(|d| &d.pieces)
            .find(|piece| piece.particles.contains(&particle))
            .map(|piece| piece.panel);
        Ok(Some(Hit {
            particle,
            position,
            panel,
        }))
    }

    /// Pointer down at (`x`, `y`): in the 3D view, on the cloth, get ready
    /// to drag it. With `Adjust` (⇧ held) the drag changes the pattern at
    /// the contour edge under the cursor: an open edge (a hem) is
    /// lengthened, a seam is taken in or let out (at the nearer end when
    /// grabbed near one). With `Move` (⌘ held) it moves the piece around
    /// the body (its placement). Otherwise the drag pulls the cloth. Returns
    /// whether there is something to drag (then moves should call `pull`);
    /// otherwise drags orbit as usual.
    #[frb(sync)]
    pub fn press(&self, view: u32, x: f64, y: f64, gesture: Gesture) -> Result<bool, ApiError> {
        let adjust = gesture == Gesture::Adjust;
        // Particles on contour edges, and what dragging them adjusts.
        let mut open: HashMap<usize, Adjust> = HashMap::new();
        if adjust {
            let ws = self.workspace()?;
            let project = ws.session().project();
            let cloth = self.cloth.lock().map_err(|_| poisoned())?;
            for (e, particles) in cloth.iter().flat_map(|d| &d.edges) {
                let n = particles.len().max(2) - 1;
                for (k, i) in particles.iter().enumerate() {
                    let tool = match project.seam_of(*e) {
                        None => Adjust::Length(*e),
                        Some(seam) => {
                            let t = k as f32 / n as f32;
                            let edge = project.edge(*e);
                            let at = match edge {
                                Some(edge) if t < SEAM_END => Some(edge.start),
                                Some(edge) if t > 1.0 - SEAM_END => Some(edge.end),
                                _ => None,
                            };
                            Adjust::Fit(seam, at)
                        }
                    };
                    open.insert(*i, tool);
                }
            }
        }
        let slot = self.graphics()?;
        let Some(v) = slot.as_ref().and_then(|g| g.views.get(&view)) else {
            return Ok(false);
        };
        if v.kind == ViewKind::Board {
            let (board_view, width, height) = (v.board_view, v.width, v.height);
            drop(slot);
            return self.press_board(view, board_view, width, height, x, y);
        }
        let only = |i: usize| !adjust || open.contains_key(&i);
        let Some(hit) = self.pick_cloth(v, &slot, x, y, &only)? else {
            return Ok(false);
        };
        drop(slot);
        if gesture == Gesture::Move {
            let placement = hit.panel.and_then(|panel| {
                let ws = self.workspace().ok()?;
                let place = ws.session().project().panel(panel)?.placement?;
                Some((panel, place))
            });
            let Some((panel, from)) = placement else {
                return Ok(false);
            };
            *self.selected()? = vec![panel];
            let running = self.simulation_status()?.running;
            *self.held.lock().map_err(|_| poisoned())? = Some(Held {
                particle: hit.particle,
                anchor: hit.position,
                tool: Tool::Move {
                    panel,
                    from,
                    running,
                },
            });
            return Ok(true);
        }
        let tool = match open.get(&hit.particle) {
            Some(adjust) => {
                // Show the panel being changed in both views.
                *self.selected()? = hit.panel.into_iter().collect();
                match *adjust {
                    Adjust::Length(edge) => Tool::Length { edge, applied: 0.0 },
                    Adjust::Fit(seam, at) => Tool::Fit {
                        seam,
                        at,
                        applied: 0.0,
                    },
                }
            }
            None => Tool::Pull { pulling: false },
        };
        *self.held.lock().map_err(|_| poisoned())? = Some(Held {
            particle: hit.particle,
            anchor: hit.position,
            tool,
        });
        Ok(true)
    }

    /// Pointer moved to (`x`, `y`) after `press`: pull the cloth there (on
    /// the plane facing the camera through the grabbed point; the
    /// simulation runs while it is held); with the Length tool, move the
    /// edge by the vertical distance dragged (down = longer); with the Fit
    /// tool, take the seam in by twice the distance dragged towards the
    /// body (away lets it out). Returns the change so far (mm) for the
    /// pattern tools.
    #[frb(sync)]
    pub fn pull(&self, view: u32, x: f64, y: f64) -> Result<Option<PatternChange>, ApiError> {
        let Some(held) = *self.held.lock().map_err(|_| poisoned())? else {
            return Ok(None);
        };
        if let Tool::Corner { .. } | Tool::Edge { .. } = held.tool {
            return self.pull_board(view, held.tool, x, y);
        }
        let anchor = held.anchor;
        let slot = self.graphics()?;
        let (Some(g), Some(v)) = (
            slot.as_ref(),
            slot.as_ref().and_then(|g| g.views.get(&view)),
        ) else {
            return Ok(None);
        };
        let (origin, dir) = g.camera.ray(x as f32, y as f32, v.width, v.height);
        let normal = g.camera.forward();
        let facing = dir.dot(normal);
        if facing.abs() < 1e-6 {
            return Ok(None);
        }
        let target = origin + dir * ((anchor - origin).dot(normal) / facing);
        drop(slot);
        let tool = match held.tool {
            Tool::Pull { pulling } => {
                if let Some(thread) = self.sim()?.as_ref() {
                    if !pulling {
                        thread.grab(held.particle);
                    }
                    thread.grab_to(target);
                }
                Tool::Pull { pulling: true }
            }
            Tool::Length { edge, applied } => {
                let total = (f64::from(anchor.y - target.y) * 1000.0).round();
                let step = serde_json::json!({ "type": "edge.lengthen", "edge": edge });
                Tool::Length {
                    edge,
                    applied: self.adjust_by(step, applied, total),
                }
            }
            Tool::Fit { seam, at, applied } => {
                // Towards the body's vertical axis takes in.
                let radius = |p: glam::Vec3| glam::Vec2::new(p.x, p.z).length();
                let total = (f64::from(radius(anchor) - radius(target)) * 2000.0).round();
                let mut step = serde_json::json!({ "type": "seam.take_in", "seam": seam });
                if let Some(at) = at {
                    step["at"] = serde_json::json!(at);
                }
                Tool::Fit {
                    seam,
                    at,
                    applied: self.adjust_by(step, applied, total),
                }
            }
            // Board tools are handled above.
            Tool::Corner { .. } | Tool::Edge { .. } => held.tool,
            Tool::Move { panel, from, .. } => {
                // Around the vertical axis by the angle swept, up or down
                // by the height dragged; the pieces start again there.
                let angle = |p: glam::Vec3| p.x.atan2(p.z).to_degrees();
                let swept = f64::from(angle(target) - angle(anchor));
                let step = serde_json::json!({
                    "type": "panel.place",
                    "panel": panel,
                    "around": from.around,
                    "angle_deg": (from.angle_deg + swept).round(),
                    "top_cm": (from.top_cm + f64::from(target.y - anchor.y) * 100.0).round().max(1.0),
                    "offset_mm": from.offset_mm,
                });
                self.run(step.to_string())?;
                self.rebuild(false)?;
                held.tool
            }
        };
        if let Some(h) = self.held.lock().map_err(|_| poisoned())?.as_mut() {
            h.tool = tool;
        }
        let change = |kind: &str, mm: f64| PatternChange {
            kind: kind.into(),
            mm,
        };
        Ok(match tool {
            Tool::Length { applied, .. } => Some(change("length", applied)),
            Tool::Fit { applied, .. } => Some(change("fit", applied)),
            Tool::Pull { .. } | Tool::Move { .. } | Tool::Corner { .. } | Tool::Edge { .. } => None,
        })
    }

    /// A press at pixel (`x`, `y`) of a board: hold the corner or else the
    /// edge under the pointer, to drag it, and select its piece. Pieces
    /// that are linked mirror copies follow their source and are not held.
    /// The board stops fitting the pattern while it changes under the
    /// pointer.
    fn press_board(
        &self,
        view: u32,
        board_view: Option<BoardView>,
        width: u32,
        height: u32,
        x: f64,
        y: f64,
    ) -> Result<bool, ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        let scene = BoardScene::from_project(project, &[]);
        let bv = board_view.unwrap_or_else(|| BoardView::fit(scene.bounds(), width, height));
        let at = bv.to_board(x, y, width, height);
        let editable = |panel: PanelId| project.panel(panel).is_some_and(|p| p.mirror_of.is_none());
        let corner = project
            .panels()
            .iter()
            .filter(|(id, _)| editable(**id))
            .flat_map(|(id, p)| {
                p.contour
                    .iter()
                    .filter_map(move |e| Some((*id, project.edge(*e)?.start)))
            })
            .filter_map(|(panel, point)| Some((panel, point, project.point(point)?.pos)))
            .filter(|(.., pos)| pos.distance(at) <= CORNER_PICK_PX * bv.mm_per_px)
            .min_by(|a, b| a.2.distance(at).total_cmp(&b.2.distance(at)));
        let (panel, edge, tool) = match corner {
            Some((panel, point, from)) => (
                panel,
                None,
                Tool::Corner {
                    point,
                    from,
                    grab: at,
                },
            ),
            None => match scene
                .edge_at(at, EDGE_PICK_PX * bv.mm_per_px)
                .filter(|e| editable(e.panel))
            {
                Some(e) => (
                    e.panel,
                    Some(e.id),
                    Tool::Edge {
                        edge: e.id,
                        grab: at,
                        applied: r3d_geom::DVec2::ZERO,
                    },
                ),
                None => return Ok(false),
            },
        };
        drop(ws);
        if let Some(v) = self
            .graphics()?
            .as_mut()
            .and_then(|g| g.views.get_mut(&view))
        {
            v.board_view = Some(bv);
        }
        *self.selected()? = vec![panel];
        *self.selected_edge.lock().map_err(|_| poisoned())? = edge;
        *self.held.lock().map_err(|_| poisoned())? = Some(Held {
            particle: 0,
            anchor: glam::Vec3::ZERO,
            tool,
        });
        Ok(true)
    }

    /// Pointer moved to (`x`, `y`) on the board while holding a corner or
    /// an edge: move it by the distance dragged, in whole mm (one undo
    /// step for the whole drag). Returns the distance moved so far.
    fn pull_board(
        &self,
        view: u32,
        tool: Tool,
        x: f64,
        y: f64,
    ) -> Result<Option<PatternChange>, ApiError> {
        let Some(at) = self
            .graphics()?
            .as_ref()
            .and_then(|g| g.views.get(&view))
            .and_then(|v| Some(v.board_view?.to_board(x, y, v.width, v.height)))
        else {
            return Ok(None);
        };
        let (step, next, moved) = match tool {
            Tool::Corner { point, from, grab } => {
                let d = (at - grab).round();
                let to = from + d;
                let step = serde_json::json!({
                    "type": "point.move", "point": point, "to": [to.x, to.y],
                });
                (step, tool, d)
            }
            Tool::Edge {
                edge,
                grab,
                applied,
            } => {
                let d = (at - grab).round();
                let by = d - applied;
                let step = serde_json::json!({
                    "type": "edge.move", "edge": edge, "by": [by.x, by.y],
                });
                (
                    step,
                    Tool::Edge {
                        edge,
                        grab,
                        applied: d,
                    },
                    d,
                )
            }
            _ => return Ok(None),
        };
        // A move the pattern refuses leaves the hold where it was.
        let moved_ok = self.run(step.to_string()).is_ok();
        if let (true, Some(h)) = (moved_ok, self.held.lock().map_err(|_| poisoned())?.as_mut()) {
            h.tool = next;
        }
        Ok(Some(PatternChange {
            kind: "move".into(),
            mm: moved.length(),
        }))
    }

    /// Run `step` with `"mm"` = `total - applied` and return the new total
    /// applied. Past a limit (a hem crossing the waist) the pattern simply
    /// stops following.
    fn adjust_by(&self, mut step: serde_json::Value, applied: f64, total: f64) -> f64 {
        if total == applied {
            return applied;
        }
        step["mm"] = serde_json::json!(total - applied);
        if self.run(step.to_string()).is_ok() {
            total
        } else {
            applied
        }
    }

    /// ⌥-click at (`x`, `y`) in the 3D view: pin the cloth there, where it
    /// is now (a `pin.add` command, so it is saved and undoable; the running
    /// simulation follows). Returns the new pin's id, or `None` when there
    /// is no cloth under the cursor.
    #[frb(sync)]
    pub fn pin_at(&self, view: u32, x: f64, y: f64) -> Result<Option<String>, ApiError> {
        let slot = self.graphics()?;
        let Some(v) = slot.as_ref().and_then(|g| g.views.get(&view)) else {
            return Ok(None);
        };
        let Some(hit) = self.pick_cloth(v, &slot, x, y, &|_| true)? else {
            return Ok(None);
        };
        drop(slot);
        let Some(panel) = hit.panel else {
            return Ok(None);
        };
        let at = self
            .cloth
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .and_then(|d| d.scene.particles.get(hit.particle))
            .map(|p| p.rest)
            .unwrap_or_default();
        let p = hit.position;
        let step = serde_json::json!({
            "type": "pin.add",
            "panel": panel,
            "at": [f64::from(at.x) * 1000.0, f64::from(at.y) * 1000.0],
            "position": [p.x, p.y, p.z],
        });
        let out: serde_json::Value = serde_json::from_str(&self.run(step.to_string())?)
            .map_err(|e| ApiError::new("internal", e.to_string()))?;
        Ok(out["created"][0].as_str().map(str::to_owned))
    }

    /// Remove a pin (`pin.remove`); the cloth there moves again.
    #[frb(sync)]
    pub fn unpin(&self, pin: String) -> Result<(), ApiError> {
        let step = serde_json::json!({ "type": "pin.remove", "pin": pin });
        self.run(step.to_string())?;
        Ok(())
    }

    /// Pointer up: let go of the cloth.
    #[frb(sync)]
    pub fn release(&self) -> Result<(), ApiError> {
        let held = self.held.lock().map_err(|_| poisoned())?.take();
        let was_pulling = held.is_some_and(|h| matches!(h.tool, Tool::Pull { pulling: true }));
        // A moved piece drapes again if the drape was running.
        if held.is_some_and(|h| matches!(h.tool, Tool::Move { running: true, .. })) {
            self.run_simulation(true)?;
        }
        // (No let-chain: the bridge macro parses this function as Rust 2021.)
        if was_pulling {
            if let Some(thread) = self.sim()?.as_ref() {
                thread.release();
            }
        }
        Ok(())
    }

    /// A drag of (`dx`, `dy`) pixels: orbit the 3D camera, pan the board.
    #[frb(sync)]
    pub fn drag(&self, view: u32, dx: f64, dy: f64) -> Result<(), ApiError> {
        let ws = self.workspace()?;
        let mut slot = self.graphics()?;
        let Some(g) = slot.as_mut() else {
            return Ok(());
        };
        let Some(v) = g.views.get_mut(&view) else {
            return Ok(());
        };
        match v.kind {
            ViewKind::Garment => g.camera.orbit(dx as f32, dy as f32),
            ViewKind::Board => {
                let scene = BoardScene::from_project(ws.session().project(), &[]);
                let mut bv = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                bv.center -= r3d_geom::DVec2::new(dx, -dy) * bv.mm_per_px;
                v.board_view = Some(bv);
            }
        }
        Ok(())
    }

    /// A scroll of `amount` (positive = zoom in) at pixel (`x`, `y`).
    #[frb(sync)]
    pub fn zoom(&self, view: u32, x: f64, y: f64, amount: f64) -> Result<(), ApiError> {
        let ws = self.workspace()?;
        let mut slot = self.graphics()?;
        let Some(g) = slot.as_mut() else {
            return Ok(());
        };
        let Some(v) = g.views.get_mut(&view) else {
            return Ok(());
        };
        match v.kind {
            ViewKind::Garment => g.camera.zoom(amount as f32),
            ViewKind::Board => {
                let scene = BoardScene::from_project(ws.session().project(), &[]);
                let mut bv = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                // Keep the board point under the cursor fixed while zooming.
                let anchor = bv.to_board(x, y, v.width, v.height);
                bv.mm_per_px = (bv.mm_per_px * (1.0 - amount * 0.1)).clamp(0.02, 50.0);
                let moved = bv.to_board(x, y, v.width, v.height);
                bv.center += anchor - moved;
                v.board_view = Some(bv);
            }
        }
        Ok(())
    }

    /// Fit the board to the pattern again.
    #[frb(sync)]
    pub fn fit_board(&self, view: u32) -> Result<(), ApiError> {
        if let Some(v) = self
            .graphics()?
            .as_mut()
            .and_then(|g| g.views.get_mut(&view))
        {
            v.board_view = None;
        }
        Ok(())
    }

    /// Look at the body from one side, framed head to toe.
    #[frb(sync)]
    pub fn look_from(&self, side: CameraSide) -> Result<(), ApiError> {
        use std::f32::consts::{FRAC_PI_2, PI};
        let yaw = match side {
            CameraSide::Front => 0.0,
            CameraSide::ThreeQuarter => OrbitCamera::default().yaw,
            CameraSide::Left => FRAC_PI_2,
            CameraSide::Back => PI,
            CameraSide::Right => -FRAC_PI_2,
        };
        if let Some(g) = self.graphics()?.as_mut() {
            g.camera = OrbitCamera::facing(yaw);
        }
        Ok(())
    }

    /// Replace the shared selection with these panel ids; unknown ids are
    /// ignored. Returns the resulting selection.
    #[frb(sync)]
    pub fn select(&self, panels: Vec<String>) -> Result<Vec<String>, ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        *self.selected_edge.lock().map_err(|_| poisoned())? = None;
        let mut selected = self.selected()?;
        *selected = panels
            .iter()
            .filter_map(|p| p.parse::<PanelId>().ok())
            .filter(|id| project.panel(*id).is_some())
            .collect();
        Ok(selected.iter().map(ToString::to_string).collect())
    }

    /// The shared selection (panel ids).
    #[frb(sync)]
    pub fn selection(&self) -> Result<Vec<String>, ApiError> {
        Ok(self.selected()?.iter().map(ToString::to_string).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_run_and_structured_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("P.r3d").to_string_lossy().into_owned();
        let engine = Engine::open(path.clone()).unwrap();
        let out = engine
            .run(r#"{"type":"panel.create_rect","width":100,"height":200}"#.into())
            .unwrap();
        assert!(out.contains("panel:2"));
        let e = engine
            .run(r#"{"type":"panel.create_rect","width":-1,"height":2}"#.into())
            .err()
            .unwrap();
        assert_eq!(e.code, "invalid.not_positive");
        // Saved: reopening sees the panel.
        drop(engine);
        let again = Engine::open(path).unwrap();
        assert!(
            again
                .run(r#"{"type":"panel.list"}"#.into())
                .unwrap()
                .contains("panel:2")
        );
    }

    #[test]
    fn views_need_attaching_first() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = Engine::open(tmp.path().join("P.r3d").to_string_lossy().into()).unwrap();
        assert_eq!(engine.render_view(1).err().unwrap().code, "no_view");
        assert_eq!(engine.click(1, 0.0, 0.0).err().unwrap().code, "no_view");
        assert!(engine.selection().unwrap().is_empty());
    }
}
