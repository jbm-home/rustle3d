//! Actions: operations that produce files instead of editing the document
//! (not undoable). They run on a [`Workspace`], which confines every write
//! to the project folder.

use r3d_io::pdf::{Paper, layout, to_pdf};
use r3d_io::{pattern_drawing, svg::to_svg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::drape;
use crate::error::{CommandError, Invalid};
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// An operation that writes files.
pub trait ActionSpec:
    crate::spec::Named + Described + Serialize + serde::de::DeserializeOwned + JsonSchema + Clone
{
    fn run(&self, ws: &Workspace) -> Result<Value, WorkspaceError>;
}

/// Default export path: `exports/<project name>.<ext>`.
fn default_path(ws: &Workspace, ext: &str) -> String {
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
    let name = if name.is_empty() { "pattern" } else { name };
    format!("exports/{name}.{ext}")
}

/// Export the pattern as SVG at 1:1 scale.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportSvg {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>.svg`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Described for ExportSvg {
    const SUMMARY: &'static str = "Export all panels as an SVG at 1:1 scale (cutting and sewing lines, notches, internal lines, grainlines, labels, 100 mm test square). Returns the written path, relative to the project folder.";
}

impl ActionSpec for ExportSvg {
    fn run(&self, ws: &Workspace) -> Result<Value, WorkspaceError> {
        let svg = to_svg(&pattern_drawing(ws.session().project()));
        let path = self.path.clone().unwrap_or_else(|| default_path(ws, "svg"));
        Ok(json!({ "path": ws.write(&path, svg.as_bytes())? }))
    }
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
}

impl Described for ExportPdf {
    const SUMMARY: &'static str = "Export all panels as a PDF at 1:1 scale, tiled on A4/Letter pages (labelled A1, A2…, with overlap guides) or on one large page. Print at 100 % and check the 100 mm test square. Returns the written path and page count.";
}

impl ActionSpec for ExportPdf {
    fn run(&self, ws: &Workspace) -> Result<Value, WorkspaceError> {
        let drawing = pattern_drawing(ws.session().project());
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
}

fn default_frames() -> u32 {
    300
}

impl Described for Simulate {
    const SUMMARY: &'static str = "Drape the placed and sewn panels on the avatar for a number of frames (1/60 s each, deterministic) and cache the result in the project, so `r3d render --view 3d` shows it. Returns the garment's extent, how open the seams are and whether the cloth has settled.";
}

impl ActionSpec for Simulate {
    fn run(&self, ws: &Workspace) -> Result<Value, WorkspaceError> {
        if !(1..=3600).contains(&self.frames) {
            return Err(CommandError::invalid("frames", Invalid::Range(1.0, 3600.0)).into());
        }
        let project = ws.session().project();
        let out = drape::simulate(project, self.frames)?;
        let cache = drape::encode_cache(drape::fingerprint(project)?, &out.positions);
        ws.write(drape::CACHE_FILE, &cache)?;
        let ys = out.positions.iter().map(|p| p.y);
        let top = ys.clone().fold(f32::MIN, f32::max);
        let bottom = ys.fold(f32::MAX, f32::min);
        let r3 = |v: f64| (v * 1000.0).round() / 1000.0;
        Ok(json!({
            "frames": out.frames,
            "simulated_s": r3(f64::from(out.frames) / 60.0),
            "particles": out.positions.len(),
            "pieces": out.drape.pieces.len(),
            "ms_per_frame": r3(out.ms_per_frame),
            "top_m": r3(top.into()),
            "bottom_m": r3(bottom.into()),
            "max_seam_gap_mm": r3(f64::from(out.stats.max_seam_gap) * 1000.0),
            "max_speed_m_s": r3(out.stats.max_speed.into()),
            "settled": out.stats.max_speed < SETTLED_SPEED,
        }))
    }
}

/// Below this particle speed (m/s) the cloth counts as settled.
const SETTLED_SPEED: f32 = 0.05;
