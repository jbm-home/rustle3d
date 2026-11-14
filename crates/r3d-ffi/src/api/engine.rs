//! The engine a UI window talks to: one project folder, plus the views the
//! UI displays (the 2D board and the 3D garment), rendered by Rust into
//! textures the UI composites without copying.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use flutter_rust_bridge::frb;
use r3d_commands::{Script, Step, Workspace};
use r3d_core::{Localizable, Message, PanelId};
use r3d_render::board::BoardView;
use r3d_render::{
    BoardRenderer, BoardScene, ClothMesh, Gpu, OrbitCamera, Target, Theme, ViewRenderer3d,
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
    dark: bool,
    started: Instant,
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
            dark: false,
            started: Instant::now(),
        })
    }
}

/// A project opened by the UI.
#[frb(opaque)]
pub struct Engine {
    ws: Mutex<Workspace>,
    graphics: Mutex<Option<Graphics>>,
    /// Selection shared by every view (not saved, not undoable).
    selection: Mutex<Vec<PanelId>>,
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

    /// Use the dark or light canvas colours.
    #[frb(sync)]
    pub fn set_dark(&self, dark: bool) -> Result<(), ApiError> {
        if let Some(g) = self.graphics()?.as_mut() {
            g.dark = dark;
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
        let mut theme = if g.dark { Theme::DARK } else { Theme::LIGHT };
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
                // Until the solver exists (M4), a demo garment stands in and
                // the selection tints it, to exercise cross-view highlighting.
                if !selected.is_empty() {
                    theme.cloth = theme.selection;
                }
                let mesh = ClothMesh::demo_skirt(100, 100, g.started.elapsed().as_secs_f32());
                g.garment.render(&g.gpu, target, &g.camera, &mesh, &theme);
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
