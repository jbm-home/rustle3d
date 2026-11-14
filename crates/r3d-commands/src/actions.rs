//! Actions: operations that produce files instead of editing the document
//! (not undoable). They run on a [`Workspace`], which confines every write
//! to the project folder.

use r3d_io::pdf::{Paper, layout, to_pdf};
use r3d_io::{pattern_drawing, svg::to_svg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
