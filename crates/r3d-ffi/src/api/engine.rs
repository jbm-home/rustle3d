//! The engine a UI window talks to: one project folder, plus the views the
//! UI displays (the 2D board and the 3D garment), rendered by Rust into
//! textures the UI composites without copying.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use flutter_rust_bridge::frb;
use std::path::PathBuf;

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_avatar::presets::{BodyPreset, PresetLibrary};
use r3d_commands::drape::{self, BodyMesh};
use r3d_commands::queries::body_model;
use r3d_commands::{Script, Step, Workspace, templates};
use r3d_sim::Solver;

use crate::sim::SimThread;
use r3d_core::{Body, Fabric, Localizable, Message, PanelId};
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
    fn new(code: &str, text: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            args: HashMap::new(),
            text: text.into(),
        }
    }
}

fn err(e: impl Localizable) -> ApiError {
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
    fn to_theme(&self) -> Result<Theme, ApiError> {
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
    _texture: wgpu::Texture,
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

/// The avatar mesh fitted to `body`.
fn avatar_mesh(body: &Body) -> Result<Mesh, ApiError> {
    let model = body_model().map_err(err)?;
    let fitted = fit(model, body);
    let mut mesh = Mesh {
        positions: fitted_positions(model, &fitted),
        normals: Vec::new(),
        indices: model.triangles().iter().flatten().copied().collect(),
    };
    mesh.recompute_normals();
    Ok(mesh)
}

/// Where each piece's particles sit in the simulation, to draw one mesh per
/// piece (own material, selection tint).
struct ClothPart {
    panel: PanelId,
    /// First particle of the piece (pieces are contiguous).
    start: usize,
    count: usize,
    /// Triangles, in indices local to the piece.
    indices: Vec<u32>,
}

/// A fabric choice for the UI.
#[derive(Debug, Clone)]
pub struct FabricSwatch {
    /// The value `garment.set_fabric` takes (`cotton`, `denim`…).
    pub id: String,
    /// Default colour, ARGB.
    pub argb: u32,
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
    /// Folder of the user's libraries (body presets…), set by the UI.
    library_dir: Mutex<Option<PathBuf>>,
    /// The simulation thread, started on first use.
    sim: Mutex<Option<SimThread>>,
    /// How to draw the loaded simulation.
    cloth: Mutex<Vec<ClothPart>>,
}

fn poisoned() -> ApiError {
    ApiError::new("internal", "engine lock poisoned")
}

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
            library_dir: Mutex::new(None),
            sim: Mutex::new(None),
            cloth: Mutex::new(Vec::new()),
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

    /// Rebuild the simulation from the document: placed pieces around the
    /// body, paused. This is also how edits reach the simulation.
    #[frb(sync)]
    pub fn reset_simulation(&self) -> Result<SimStatus, ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        let model = body_model().map_err(err)?;
        let body = project
            .avatar()
            .map(|a| fitted_positions(model, &fit(model, &a.body)));
        let drape = drape::build(
            project,
            body.as_deref().map(|positions| BodyMesh {
                positions,
                triangles: model.triangles(),
            }),
            drape::DEFAULT_SPACING_MM,
        )
        .map_err(err)?;
        drop(ws);
        let mut parts: Vec<ClothPart> = Vec::new();
        for (i, panel) in drape.particle_panels.iter().enumerate() {
            match parts.last_mut() {
                Some(p) if p.panel == *panel => p.count += 1,
                _ => parts.push(ClothPart {
                    panel: *panel,
                    start: i,
                    count: 1,
                    indices: Vec::new(),
                }),
            }
        }
        for t in &drape.scene.triangles {
            let first = t.vertices[0] as usize;
            if let Some(p) = parts
                .iter_mut()
                .find(|p| (p.start..p.start + p.count).contains(&first))
            {
                p.indices
                    .extend(t.vertices.iter().map(|v| *v - p.start as u32));
            }
        }
        let sim = self.sim()?;
        let thread = sim.as_ref().ok_or_else(poisoned)?;
        if parts.is_empty() {
            thread.clear();
        } else {
            thread.load(Solver::new(drape.scene));
        }
        *self.cloth.lock().map_err(|_| poisoned())? = parts;
        drop(sim);
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
        if ws.session().is_dirty() {
            ws.save().map_err(err)?;
        }
        // Forget selected panels that no longer exist (e.g. after undo).
        let project = ws.session().project();
        self.selected()?.retain(|id| project.panel(*id).is_some());
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

    /// Run a built-in template (a starting project, see
    /// `r3d_commands::templates`) and return its results like `run_script`.
    #[frb(sync)]
    pub fn run_template(&self, name: String) -> Result<String, ApiError> {
        let script = templates::get(&name).ok_or_else(|| ApiError {
            code: "not_found".into(),
            args: HashMap::from([("id".into(), name.clone())]),
            text: format!("no template called {name}"),
        })?;
        self.run_script(script.to_owned())
    }

    /// Attach a view rendering into a UI texture (`surface`: platform handle
    /// from the UI side, an `IOSurfaceRef` pointer on macOS). Returns the
    /// view id.
    #[frb(sync)]
    pub fn attach_view(
        &self,
        kind: ViewKind,
        surface: u64,
        width: u32,
        height: u32,
    ) -> Result<u32, ApiError> {
        let mut slot = self.graphics()?;
        if slot.is_none() {
            *slot = Some(Graphics::new()?);
        }
        let g = slot.as_mut().ok_or_else(poisoned)?;
        let texture = crate::surface::wrap_ui_surface(&g.gpu.device, surface, width, height)
            .map_err(|e| ApiError::new("ui_surface", e))?;
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id = g.next_view;
        g.next_view += 1;
        g.views.insert(
            id,
            View {
                kind,
                _texture: texture,
                target,
                width,
                height,
                board_view: None,
            },
        );
        Ok(id)
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
                let scene = BoardScene::from_project(project, &selected);
                let board_view = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                g.board.render(&g.gpu, target, &scene, &board_view, &theme);
            }
            ViewKind::Garment => {
                let body = project.avatar().map(|a| a.body);
                if body != g.avatar.as_ref().map(|(b, _)| *b) {
                    g.avatar = match body {
                        Some(b) => Some((b, avatar_mesh(&b)?)),
                        None => None,
                    };
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
                let pieces: Vec<(PanelId, Mesh)> = cloth
                    .iter()
                    .filter_map(|part| {
                        let positions = published
                            .positions
                            .get(part.start..part.start + part.count)?;
                        let mut mesh = Mesh {
                            positions: positions.to_vec(),
                            normals: Vec::new(),
                            indices: part.indices.clone(),
                        };
                        mesh.recompute_normals();
                        Some((part.panel, mesh))
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
        let mut selected = self.selected()?;
        selected.clear();
        match v.kind {
            ViewKind::Board => {
                let scene = BoardScene::from_project(project, &[]);
                let board_view = v
                    .board_view
                    .unwrap_or_else(|| BoardView::fit(scene.bounds(), v.width, v.height));
                let p = board_view.to_board(x, y, v.width, v.height);
                selected.extend(scene.panel_at(p));
            }
            // Picking on the real garment comes with the solver (M8 tools);
            // for now a click selects the whole garment.
            ViewKind::Garment => selected.extend(project.panels().keys().copied()),
        }
        Ok(selected.iter().map(ToString::to_string).collect())
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

    /// Replace the shared selection with these panel ids; unknown ids are
    /// ignored. Returns the resulting selection.
    #[frb(sync)]
    pub fn select(&self, panels: Vec<String>) -> Result<Vec<String>, ApiError> {
        let ws = self.workspace()?;
        let project = ws.session().project();
        let mut selected = self.selected()?;
        *selected = panels
            .iter()
            .filter_map(|p| p.parse::<PanelId>().ok())
            .filter(|id| project.panel(*id).is_some())
            .collect();
        Ok(selected.iter().map(ToString::to_string).collect())
    }

    /// Where the user's libraries (body presets…) are stored.
    #[frb(sync)]
    pub fn set_library_dir(&self, path: String) -> Result<(), ApiError> {
        *self.library_dir.lock().map_err(|_| poisoned())? = Some(PathBuf::from(path));
        Ok(())
    }

    fn presets_file(&self) -> Result<PathBuf, ApiError> {
        self.library_dir
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .map(|d| d.join("body-presets.json"))
            .ok_or_else(|| ApiError::new("no_library", "the library folder is not set"))
    }

    fn library(&self) -> Result<PresetLibrary, ApiError> {
        PresetLibrary::load(&self.presets_file()?)
            .map_err(|e| ApiError::new("library", e.to_string()))
    }

    /// Body presets as JSON: `[{name, body, builtin}]`, standard sizes first,
    /// then the user's own.
    #[frb(sync)]
    pub fn body_presets(&self) -> Result<String, ApiError> {
        let tag = |p: BodyPreset, builtin: bool| serde_json::json!({ "name": p.name, "body": p.body, "builtin": builtin });
        let mut all: Vec<serde_json::Value> = r3d_avatar::presets::builtin()
            .into_iter()
            .map(|p| tag(p, true))
            .collect();
        all.extend(self.library()?.presets.into_iter().map(|p| tag(p, false)));
        Ok(serde_json::Value::Array(all).to_string())
    }

    /// Save (or replace) one of the user's body presets. `body_json` is a
    /// `Body` object as in the `avatar.set` command.
    #[frb(sync)]
    pub fn save_body_preset(&self, name: String, body_json: String) -> Result<(), ApiError> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(ApiError::new("invalid.empty", "the preset needs a name"));
        }
        let body: Body = serde_json::from_str(&body_json)
            .map_err(|e| ApiError::new("malformed", e.to_string()))?;
        let mut lib = self.library()?;
        lib.upsert(BodyPreset { name, body });
        lib.save(&self.presets_file()?)
            .map_err(|e| ApiError::new("library", e.to_string()))
    }

    /// Delete one of the user's body presets.
    #[frb(sync)]
    pub fn delete_body_preset(&self, name: String) -> Result<(), ApiError> {
        let mut lib = self.library()?;
        if lib.remove(&name) {
            lib.save(&self.presets_file()?)
                .map_err(|e| ApiError::new("library", e.to_string()))?;
        }
        Ok(())
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
    fn body_presets_are_listed_saved_and_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = Engine::open(tmp.path().join("P.r3d").to_string_lossy().into()).unwrap();
        assert_eq!(engine.body_presets().err().unwrap().code, "no_library");
        engine
            .set_library_dir(tmp.path().join("lib").to_string_lossy().into())
            .unwrap();
        let list = |e: &Engine| -> Vec<serde_json::Value> {
            serde_json::from_str(&e.body_presets().unwrap()).unwrap()
        };
        let builtin = list(&engine).len();
        assert!(builtin >= 9);
        let body = r#"{"masculinity":0,"height_cm":165,"bust_cm":88,"waist_cm":70,"hips_cm":96}"#;
        engine.save_body_preset("Me".into(), body.into()).unwrap();
        let all = list(&engine);
        assert_eq!(all.len(), builtin + 1);
        assert_eq!(all.last().unwrap()["builtin"], false);
        assert_eq!(
            engine
                .save_body_preset(" ".into(), body.into())
                .err()
                .unwrap()
                .code,
            "invalid.empty"
        );
        engine.delete_body_preset("Me".into()).unwrap();
        assert_eq!(list(&engine).len(), builtin);
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
