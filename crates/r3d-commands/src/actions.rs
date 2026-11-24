//! Actions: operations that produce files instead of editing the document
//! (not undoable). They run on a [`Workspace`], which confines every write
//! to the project folder.

use r3d_io::gltf::to_glb;
use r3d_io::obj::to_obj;
use r3d_io::pdf::{Paper, layout, to_pdf};
use r3d_io::{Drawing, graded_drawing, pattern_drawing, svg::to_svg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use r3d_core::AnyId;

use crate::commands::{PatternImport, PieceSpec};
use crate::drape;
use crate::error::{CommandError, Invalid};
use crate::progress::Progress;
use crate::registry::Command;
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// An operation that writes files.
pub trait ActionSpec:
    crate::spec::Named + Described + Serialize + serde::de::DeserializeOwned + JsonSchema + Clone
{
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError>;

    /// [`ActionSpec::run`], reporting to `progress` and stopping when it
    /// asks: for actions that take seconds. The default ignores it.
    fn run_with(&self, ws: &mut Workspace, _progress: &Progress) -> Result<Value, WorkspaceError> {
        self.run(ws)
    }
}

/// Default export path: `exports/<project name>.<ext>`.
fn default_path(ws: &Workspace, ext: &str) -> String {
    format!("exports/{}.{ext}", export_stem(ws))
}

/// The project's name as a file name: characters other than letters,
/// digits, `-`, `_` and spaces replaced, `pattern` when nothing is left.
pub(crate) fn export_stem(ws: &Workspace) -> String {
    let name: String = ws
        .session()
        .project()
        .name()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_ ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "pattern".to_owned()
    } else {
        name.to_owned()
    }
}

/// Export the pattern as SVG at 1:1 scale.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportSvg {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.svg`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Sizes of the range to export, by name: one size alone, or several
    /// nested (a graded nest). Default: the pattern as drawn (base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
}

impl Described for ExportSvg {
    const SUMMARY: &'static str = "Export all panels as an SVG at 1:1 scale (cutting and sewing lines, notches, internal lines, grainlines, labels, 100 mm test square). With `sizes`, the pattern in those sizes of the range: one alone, or a graded nest (the base size in full, the others' cutting lines in colour with a legend). Returns the written path, relative to the project folder.";
}

impl ActionSpec for ExportSvg {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let svg = to_svg(&drawing(ws, self.sizes.as_deref())?);
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "svg"));
        Ok(json!({ "path": ws.write(&path, svg.as_bytes())? }))
    }
}

/// The indices of the sizes called `names`, in the range's order.
fn size_indices(project: &r3d_core::Project, names: &[String]) -> Result<Vec<usize>, CommandError> {
    if names.is_empty() {
        return Err(CommandError::invalid("sizes", Invalid::Empty));
    }
    let mut out = names
        .iter()
        .map(|n| crate::commands::size_index(project, n.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// The pattern drawing of an export: in `sizes`, or as drawn.
fn drawing(ws: &Workspace, sizes: Option<&[String]>) -> Result<Drawing, CommandError> {
    let project = ws.session().project();
    Ok(match sizes {
        Some(names) => graded_drawing(project, &size_indices(project, names)?),
        None => pattern_drawing(project),
    })
}

/// Page layout of a PDF export.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaperSpec {
    /// A4 tiles to print at home.
    #[default]
    A4,
    /// US Letter tiles to print at home.
    Letter,
    /// One page as large as the pattern (plotter, print shop).
    Single,
}

impl From<PaperSpec> for Paper {
    fn from(p: PaperSpec) -> Paper {
        match p {
            PaperSpec::A4 => Paper::A4,
            PaperSpec::Letter => Paper::Letter,
            PaperSpec::Single => Paper::Single,
        }
    }
}

/// Export the pattern as a PDF at 1:1 scale.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPdf {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.pdf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `a4` (default) or `letter` tiles with overlap and assembly guides,
    /// or `single` for one large page.
    #[serde(default)]
    pub paper: PaperSpec,
    /// Sizes of the range to export, by name: one size alone, or several
    /// nested (a graded nest). Default: the pattern as drawn (base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
}

impl Described for ExportPdf {
    const SUMMARY: &'static str = "Export all panels as a PDF at 1:1 scale, tiled on A4/Letter pages (labelled A1, A2…, with overlap guides) or on one large page. Print at 100 % and check the 100 mm test square. With `sizes`, the pattern in those sizes of the range: one alone, or a graded nest. Returns the written path and page count.";
}

impl ActionSpec for ExportPdf {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let drawing = drawing(ws, self.sizes.as_deref())?;
        let paper = Paper::from(self.paper);
        let pages = layout(&drawing, paper).2.len();
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "pdf"));
        let written = ws.write(&path, &to_pdf(&drawing, paper))?;
        Ok(json!({ "path": written, "pages": pages }))
    }
}

/// Drape the garment headless and cache the result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Simulate {
    /// Frames of 1/60 s to simulate (1–3600, default 300 = 5 s).
    #[serde(default = "default_frames")]
    pub frames: u32,
    /// A motion of the avatar, started after 1.5 s of draping (once the
    /// garment has closed); the cloth follows it. Omitted: the avatar
    /// stands still.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<AvatarMotion>,
    /// Instead of `motion`: a BVH file of a motion captured on a person,
    /// relative to the project folder, retargeted to the avatar and
    /// played in a loop on the spot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion_file: Option<String>,
}

/// A motion of the avatar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AvatarMotion {
    /// Walking on the spot (a 1.15 s step cycle).
    Walk,
    /// Half a turn to the left and back (6 s).
    Turn,
    /// Both arms raised overhead and lowered (4 s).
    RaiseArms,
    /// Standing at rest: the weight shifting from leg to leg, breathing
    /// (8 s).
    Idle,
}

impl From<AvatarMotion> for r3d_avatar::motion::Motion {
    fn from(m: AvatarMotion) -> Self {
        match m {
            AvatarMotion::Walk => Self::Walk,
            AvatarMotion::Turn => Self::Turn,
            AvatarMotion::RaiseArms => Self::RaiseArms,
            AvatarMotion::Idle => Self::Idle,
        }
    }
}

fn default_frames() -> u32 {
    300
}

impl Described for Simulate {
    const SUMMARY: &'static str = "Drape the placed and sewn panels on the avatar for a number of frames (1/60 s each, deterministic) and cache the result in the project, so `r3d render --view 3d` shows it. With `motion`, the avatar walks, turns or raises its arms after 1.5 s and the cloth follows (or, with `motion_file`, plays a motion captured on a person from a BVH file in the project folder); renders and exports then show the body and the garment as they are at the last frame. Returns the garment's extent, how open the seams are, how many points buttons and closed zippers hold and how far apart they still are, each elastic's length on the cloth against its relaxed length, and whether the cloth has settled.";
}

impl ActionSpec for Simulate {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        self.run_with(ws, &Progress::default())
    }

    fn run_with(&self, ws: &mut Workspace, progress: &Progress) -> Result<Value, WorkspaceError> {
        if !(1..=3600).contains(&self.frames) {
            return Err(CommandError::invalid("frames", Invalid::Range(1.0, 3600.0)).into());
        }
        let project = ws.session().project();
        let imported = project.avatar().and_then(|a| a.mesh.as_ref());
        if imported.is_some() && (self.motion.is_some() || self.motion_file.is_some()) {
            return Err(CommandError::ImportedBodyStill.into());
        }
        let file = match &self.motion_file {
            Some(_) if self.motion.is_some() => {
                return Err(CommandError::invalid("motion_file", Invalid::WithMotion).into());
            }
            Some(path) => Some((path.as_str(), ws.read(path)?)),
            None => None,
        };
        let motion = match &file {
            Some((path, bytes)) => Some(drape::captured_motion(path, bytes)?),
            None => self.motion.map(Into::into),
        };
        let project = ws.session().project();
        let body_warnings = imported.map(|m| crate::bodies::check(m).warnings());
        let out = drape::simulate(project, self.frames, motion, progress)?;
        let cache = drape::encode_cache(
            drape::fingerprint(project)?,
            &out,
            file.as_ref().map(|(p, b)| (*p, b.as_slice())),
        );
        ws.write(drape::CACHE_FILE, &cache)?;
        let ys = out.positions.iter().map(|p| p.y);
        let top = ys.clone().fold(f32::MIN, f32::max);
        let bottom = ys.fold(f32::MAX, f32::min);
        let r3 = |v: f64| (v * 1000.0).round() / 1000.0;
        let elastics: Vec<Value> = out
            .drape
            .elastic_lengths(&out.positions)
            .into_iter()
            .map(|(id, on, rest)| {
                json!({
                    "elastic": id,
                    "length_mm": r3(f64::from(on) * 1000.0),
                    "relaxed_mm": r3(f64::from(rest) * 1000.0),
                })
            })
            .collect();
        Ok(json!({
            "frames": out.frames,
            "simulated_s": r3(f64::from(out.frames) / 60.0),
            "particles": out.positions.len(),
            "pieces": out.drape.pieces.len(),
            "ms_per_frame": r3(out.ms_per_frame),
            "top_m": r3(top.into()),
            "bottom_m": r3(bottom.into()),
            "max_seam_gap_mm": r3(f64::from(out.stats.max_seam_gap) * 1000.0),
            "closures": out.drape.scene.closures.len(),
            "max_closure_gap_mm": r3(f64::from(out.stats.max_closure_gap) * 1000.0),
            "elastics": elastics,
            "max_speed_m_s": r3(out.stats.max_speed.into()),
            "settled": out.stats.max_speed < SETTLED_SPEED,
            "motion": out.animation.as_ref().map(|a| a.motion.name()),
            "motion_s": out
                .animation
                .as_ref()
                .map(|a| r3((f64::from(out.frames) / 60.0 - f64::from(a.start)).max(0.0))),
            "body_warnings": body_warnings,
        }))
    }
}

/// Drape on a body imported from an OBJ, glTF or GLB file.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarImport {
    /// The file, relative to the project folder (`r3d avatar import` and
    /// the app copy files into `avatars/`): `.obj`, `.gltf` (buffers
    /// embedded) or `.glb`.
    pub path: String,
    /// Metres per file unit; detected when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// The file's up axis (`y`, `z`, `x`, `-y`, `-z`, `-x`); detected
    /// when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<r3d_core::UpAxis>,
    /// Turn about the vertical (degrees) so the body faces +Z; detected
    /// when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_deg: Option<f64>,
}

impl Described for AvatarImport {
    const SUMMARY: &'static str = "Drape on a body imported from an OBJ, glTF or GLB file inside the project folder (a scan, a game character; you own its licence). Units, up axis and facing are detected unless given (correct them with avatar.orient); the arrangement points are guessed (move them with avatar.set_point) and the measurements taken on the body. One undo step (`avatar.import_mesh`). Returns the measurements and the check (as avatar.check). Imported bodies do not move.";
}

impl ActionSpec for AvatarImport {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let format = r3d_io::model::ModelFormat::of_name(&self.path)
            .ok_or_else(|| CommandError::invalid("path", Invalid::Format))?;
        let bytes = ws.read(&self.path)?;
        let loaded = crate::bodies::register(&bytes, format)?;
        let cmd = Command::AvatarImportMesh(crate::commands::AvatarImportMesh {
            file: self.path.clone(),
            hash: loaded.hash.clone(),
            scale: self.scale,
            up: self.up,
            turn_deg: self.turn_deg,
        });
        ws.session_mut().execute(&cmd)?;
        let avatar = ws.session().project().avatar();
        Ok(json!({
            "body": avatar.map(|a| a.body),
            "check": avatar.and_then(|a| a.mesh.as_ref()).map(crate::bodies::check),
        }))
    }
}

/// The draped garment to export: the cached drape if it is up to date,
/// otherwise the starting positions; which one, for the result.
fn draped(
    ws: &Workspace,
    avatar: bool,
) -> Result<(r3d_io::scene3d::Scene3d, &'static str), WorkspaceError> {
    let project = ws.session().project();
    let latest = drape::Latest::load(ws.dir(), project)?;
    let body = latest.body.as_deref().filter(|_| avatar);
    let avatar = body.zip(latest.form.as_ref().map(|f| &f.triangles[..]));
    Ok((
        drape::scene3d(
            project,
            &latest.drape,
            &latest.positions,
            avatar,
            &crate::prints::Prints::default().all(project, Some(ws.dir())),
        ),
        latest.state(),
    ))
}

/// Export the draped garment as glTF 2.0 (GLB).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportGltf {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.glb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Include the avatar as its own node (default false).
    #[serde(default)]
    pub avatar: bool,
}

impl Described for ExportGltf {
    const SUMMARY: &'static str = "Export the draped garment as glTF 2.0 binary (GLB): one named node and mesh per piece, metres, Y up, texture coordinates from the pattern along each piece's grainline (in repeats of the garment's print, embedded as its base colour texture; 1 unit = 1 m of fabric without one), the fabric as a PBR material with KHR_materials_sheen; the avatar too if asked. Uses the cached drape from `simulate` if it is up to date, otherwise the starting positions, and says which. Returns the written path.";
}

impl ActionSpec for ExportGltf {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let (scene, state) = draped(ws, self.avatar)?;
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "glb"));
        let written = ws.write(&path, &to_glb(&scene))?;
        Ok(json!({ "path": written, "drape": state, "pieces": scene.pieces.len() }))
    }
}

/// Export the draped garment as Wavefront OBJ.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportObj {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.obj`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Include the avatar as its own object (default false).
    #[serde(default)]
    pub avatar: bool,
}

impl Described for ExportObj {
    const SUMMARY: &'static str = "Export the draped garment as Wavefront OBJ: one object per piece, metres, Y up, texture coordinates from the pattern along each piece's grainline, smooth normals; with an MTL file beside it (one material per garment, its colour, and its print's image as `map_Kd`, written beside it too). Uses the cached drape from `simulate` if it is up to date, otherwise the starting positions, and says which. Returns the written path.";
}

impl ActionSpec for ExportObj {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let (scene, state) = draped(ws, self.avatar)?;
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "obj"));
        let (folder, stem) = match path.rsplit_once('/') {
            Some((folder, name)) => (format!("{folder}/"), name),
            None => (String::new(), path.as_str()),
        };
        let stem = stem.strip_suffix(".obj").unwrap_or(stem);
        let files = to_obj(&scene, stem);
        let written = ws.write(&path, files.obj.as_bytes())?;
        let mtl = ws.write(
            &format!("{folder}{}.mtl", stem.replace(char::is_whitespace, "_")),
            files.mtl.as_bytes(),
        )?;
        let textures = files
            .textures
            .iter()
            .map(|(name, bytes)| ws.write(&format!("{folder}{name}"), bytes))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({
            "path": written,
            "mtl": mtl,
            "textures": textures,
            "drape": state,
            "pieces": scene.pieces.len(),
        }))
    }
}

/// Export the pattern as DXF-AAMA.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportDxf {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.dxf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Sizes of the range to export, by name: one size alone, or several
    /// nested (a graded nest). Default: the pattern as drawn (base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
}

impl Described for ExportDxf {
    const SUMMARY: &'static str = "Export all panels as DXF-AAMA (ASTM D6673, ASCII DXF in mm), for pattern and cutting software: one block per piece with its cutting line, sewing line, turn points, notches, grainline, internal lines and name. With `sizes`, a graded nest: every piece once per size, with a `Size:` text. Returns the written path.";
}

impl ActionSpec for ExportDxf {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let project = ws.session().project();
        let dxf = match self.sizes.as_deref() {
            Some(names) => r3d_io::dxf::to_dxf_sizes(project, &size_indices(project, names)?),
            None => r3d_io::dxf::to_dxf(project),
        };
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "dxf"));
        Ok(json!({ "path": ws.write(&path, dxf.as_bytes())? }))
    }
}

/// What one unit of an imported drawing is.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UnitSpec {
    Mm,
    Cm,
    M,
    /// Inch.
    In,
    /// Point, 1/72 inch.
    Pt,
    /// CSS pixel, 1/96 inch.
    Px,
}

impl UnitSpec {
    fn mm(self) -> f64 {
        match self {
            UnitSpec::Mm => 1.0,
            UnitSpec::Cm => 10.0,
            UnitSpec::M => 1000.0,
            UnitSpec::In => 25.4,
            UnitSpec::Pt => 25.4 / 72.0,
            UnitSpec::Px => 25.4 / 96.0,
        }
    }
}

/// Import a pattern from a DXF-AAMA, SVG, PDF, Seamly2D or Valentina
/// file.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportPattern {
    /// The file, relative to the project folder (`r3d import` and the app
    /// copy files into `imports/`). Its extension gives the format:
    /// `.dxf`, `.svg`, `.pdf`, or a parametric pattern `.sm2d` (Seamly2D)
    /// or `.val` (Valentina).
    pub path: String,
    /// A parametric pattern's measurement file, relative to the project
    /// folder. By default the one the pattern names, looked for next to
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements: Option<String>,
    /// What one unit of the drawing is, when the file says nothing or says
    /// it wrong: `mm`, `cm`, `m`, `in`, `pt` or `px`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<UnitSpec>,
    /// Size factor for a drawing at another scale (e.g. 2 for a pattern
    /// drawn at half size). Default 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// PDF: import only this page (1 = first). By default every page,
    /// with the tiles of a tiled pattern put back together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

impl Described for ImportPattern {
    const SUMMARY: &'static str = "Import the pieces of a pattern drawn elsewhere, from a DXF-AAMA (or plain DXF), SVG or PDF file inside the project folder: outlines (curves fitted back to editable edges), seam allowances, notches, grainlines, internal lines and names. A tiled PDF's pages are put back together. One undo step (`pattern.import`). Returns the new panels, what was left out, and the scale: `file`, `given` or `assumed` (check the size when assumed). A Seamly2D (.sm2d) or Valentina (.val) pattern is drafted with its measurement file (the one it names, next to it, or `measurements`) and becomes a garment that can be drafted again for other bodies and sizes (`pattern.import_parametric`); it returns the garment and panels, the pieces left out and the drafting steps not supported.";
}

impl ActionSpec for ImportPattern {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        use r3d_io::import::ImportOptions;
        if let Some(s) = self.scale {
            crate::error::positive("scale", s)?;
            crate::error::finite("scale", &[s])?;
        }
        let ext = std::path::Path::new(&self.path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        let bytes = ws.read(&self.path)?;
        if matches!(ext.as_str(), "val" | "sm2d") {
            return self.parametric(ws, &bytes);
        }
        let options = ImportOptions {
            mm_per_unit: self.units.map(UnitSpec::mm),
            page: self.page.map(|p| p as usize),
            scale: self.scale,
        };
        let pattern = match ext.as_str() {
            "dxf" => r3d_io::dxf::read(&bytes, &options)?,
            "svg" => r3d_io::import::svg::read(&bytes, &options)?,
            "pdf" => r3d_io::import::pdf::read(&bytes, &options)?,
            _ => return Err(CommandError::invalid("path", Invalid::Format).into()),
        };
        let pieces = pattern
            .pieces
            .iter()
            .map(PieceSpec::from_imported)
            .collect();
        let cmd = Command::PatternImport(PatternImport {
            pieces,
            keep_position: false,
        });
        let out = ws.session_mut().execute(&cmd)?;
        let panels: Vec<String> = out
            .created
            .iter()
            .filter(|id| matches!(id, AnyId::Panel(_)))
            .map(ToString::to_string)
            .collect();
        Ok(json!({
            "panels": panels,
            "ignored": pattern.ignored,
            "mm_per_unit": pattern.mm_per_unit,
            "scale": pattern.scale.as_str(),
        }))
    }
}

impl ImportPattern {
    /// A Seamly2D or Valentina pattern, with its measurement file.
    fn parametric(&self, ws: &mut Workspace, bytes: &[u8]) -> Result<Value, WorkspaceError> {
        let pattern = text(bytes)?;
        let link = r3d_io::import::val::linked_measurements(bytes).map_err(CommandError::from)?;
        let measurements = match (&self.measurements, link) {
            (Some(path), _) => Some(text(&ws.read(path)?)?),
            (None, Some(link)) => Some(text(&beside(ws, &self.path, &link)?)?),
            (None, None) => None,
        };
        let cmd = Command::PatternImportParametric(crate::commands::PatternImportParametric {
            file: self.path.clone(),
            pattern,
            measurements,
        });
        let out = ws.session_mut().execute(&cmd)?;
        let garment = out.created.iter().find_map(|id| match id {
            AnyId::Garment(g) => Some(*g),
            _ => None,
        });
        let panels: Vec<String> = out
            .created
            .iter()
            .filter(|id| matches!(id, AnyId::Panel(_)))
            .map(ToString::to_string)
            .collect();
        // What was left out, from the pattern as drafted.
        let project = ws.session().project();
        let source = out
            .created
            .iter()
            .find_map(|id| match id {
                AnyId::Panel(p) => project.panel(*p),
                _ => None,
            })
            .and_then(|p| project.garments().get(&p.garment))
            .and_then(|g| g.source.clone());
        let (ignored, unsupported) = match source {
            Some(s) => crate::sources::report(&s)?,
            None => (0, Vec::new()),
        };
        Ok(json!({
            "garment": garment.map(|g| g.to_string()),
            "panels": panels,
            "ignored": ignored,
            "unsupported": unsupported,
        }))
    }
}

/// A file's text.
fn text(bytes: &[u8]) -> Result<String, WorkspaceError> {
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        CommandError::from(r3d_io::import::val::ValError::Malformed {
            detail: "not UTF-8 text".into(),
        })
        .into()
    })
}

/// The measurement file `link` that a pattern at `pattern` names: at that
/// path from the pattern's folder, else by its name next to the pattern.
fn beside(ws: &Workspace, pattern: &str, link: &str) -> Result<Vec<u8>, WorkspaceError> {
    let folder = pattern.rsplit_once('/').map_or("", |(f, _)| f);
    let name = link.rsplit('/').next().unwrap_or(link);
    let join = |p: &str| {
        if folder.is_empty() {
            p.to_owned()
        } else {
            format!("{folder}/{p}")
        }
    };
    ws.read(&join(link))
        .or_else(|_| ws.read(&join(name)))
        .map_err(|_| {
            CommandError::from(r3d_io::import::val::ValError::MissingMeasurementFile {
                file: link.to_owned(),
            })
            .into()
        })
}

/// Take the avatar's body and sizes from a Seamly2D or Valentina
/// measurement file.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportMeasurements {
    /// The file, relative to the project folder: `.vit` or `.smis`
    /// (individual), `.vst` or `.smms` (multisize).
    pub path: String,
}

impl Described for ImportMeasurements {
    const SUMMARY: &'static str = "Read a Seamly2D or Valentina measurement file inside the project folder (.vit, .smis, .vst, .smms): the avatar takes its height, bust, waist and hips, its other measurements become variables, and a multisize file's sizes become the size range. One undo step (`avatar.use_measurements`). Returns the body, the sizes and the number of variables.";
}

impl ActionSpec for ImportMeasurements {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        self.run_with(ws, &Progress::default())
    }

    fn run_with(&self, ws: &mut Workspace, progress: &Progress) -> Result<Value, WorkspaceError> {
        let measurements = text(&ws.read(&self.path)?)?;
        let cmd =
            Command::AvatarUseMeasurements(crate::commands::AvatarUseMeasurements { measurements });
        ws.session_mut().execute_with(&cmd, progress)?;
        let p = ws.session().project();
        Ok(json!({
            "body": p.avatar().map(|a| a.body),
            "sizes": p
                .grading()
                .map(|g| g.sizes.iter().map(|s| s.name.clone()).collect::<Vec<_>>()),
            "variables": p.variables().len(),
        }))
    }
}

/// Below this particle speed (m/s) the cloth counts as settled.
const SETTLED_SPEED: f32 = 0.05;
