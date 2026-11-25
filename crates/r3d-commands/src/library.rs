//! The user's pattern library (docs/specs/32): garments saved from a
//! project to add to others. It lives in the user's library folder,
//! shared by every project, next to the body presets and motions; the
//! prints' and fabrics' libraries will sit beside it in the same way.
//!
//! A library pattern is a small project folder, `patterns/<Name>.r3d/`,
//! holding the garment alone with the body it was drafted for (its size
//! range too), so it migrates, opens, renders and packs like any
//! project; `library.json` beside its `project.json` says its kind.

use std::fs;
use std::path::{Path, PathBuf};

use r3d_core::file::FileError;
use r3d_core::project_dir::{self, PROJECT_FILE};
use r3d_core::{GarmentId, PrintSource, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::ActionSpec;
use crate::commands::{GarmentRemove, PatternAdd};
use crate::error::{CommandError, Invalid};
use crate::progress::Progress;
use crate::projects;
use crate::registry::Command;
use crate::session::Session;
use crate::spec::Described;
use crate::step::Step;
use crate::templates::Template;
use crate::workspace::{Workspace, WorkspaceError};

/// The folder of library patterns in the user's library folder.
const PATTERNS: &str = "patterns";
/// A library pattern's own file: its kind.
const META_FILE: &str = "library.json";

/// What a pattern is, to sort the library.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    #[default]
    Tops,
    Bottoms,
    DressesAndSkirts,
    Accessories,
}

/// A library pattern's own file.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Meta {
    #[serde(default)]
    kind: PatternKind,
}

/// A pattern of the user's library, as listed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LibraryPattern {
    pub name: String,
    pub kind: PatternKind,
    /// Its folder (a project folder: the app draws its thumbnail from it).
    pub path: PathBuf,
    /// The template it was drafted from (`t_shirt`…), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// The pattern library in a user's library folder.
#[derive(Clone, Debug)]
pub struct PatternLibrary {
    dir: PathBuf,
}

fn write_err(path: &Path, e: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::Write {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

impl PatternLibrary {
    /// The pattern library of the user's library folder `library_dir`.
    pub fn new(library_dir: &Path) -> Self {
        Self {
            dir: library_dir.join(PATTERNS),
        }
    }

    /// Every pattern, by name (case aside); folders that do not open are
    /// left out.
    pub fn list(&self) -> Vec<LibraryPattern> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<LibraryPattern> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().join(PROJECT_FILE).is_file())
            .filter_map(|e| Self::read(&e.path()))
            .collect();
        out.sort_by_key(|p| p.name.to_lowercase());
        out
    }

    fn read(dir: &Path) -> Option<LibraryPattern> {
        let project = project_dir::load(dir).ok()?.project;
        let meta: Meta = fs::read(dir.join(META_FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let template = project
            .garments()
            .values()
            .find_map(|g| g.template.as_ref().map(|t| t.name.clone()));
        Some(LibraryPattern {
            name: project.name().to_owned(),
            kind: meta.kind,
            path: dir.to_path_buf(),
            template,
        })
    }

    /// The pattern called `name` (case aside).
    pub fn get(&self, name: &str) -> Result<LibraryPattern, WorkspaceError> {
        let key = name.trim().to_lowercase();
        self.list()
            .into_iter()
            .find(|p| p.name.to_lowercase() == key)
            .ok_or_else(|| WorkspaceError::NotInLibrary {
                name: name.to_owned(),
            })
    }

    /// The project document of the pattern called `name`, as its file
    /// holds it (any schema version), and its folder.
    pub fn document(&self, name: &str) -> Result<(Value, PathBuf), WorkspaceError> {
        let dir = self.get(name)?.path;
        let file = dir.join(PROJECT_FILE);
        let text = fs::read_to_string(&file).map_err(|e| WorkspaceError::Read {
            path: file.display().to_string(),
            detail: e.to_string(),
        })?;
        let doc = serde_json::from_str(&text).map_err(|e| FileError::Json {
            detail: e.to_string(),
        })?;
        Ok((doc, dir))
    }

    /// Save `garment` of `project` (whose folder is `project_dir`) as
    /// the pattern `name`: the garment alone, with the project's body,
    /// size range and variables, and the print image it uses. A pattern
    /// of that name is replaced only if `replace`.
    pub fn save(
        &self,
        project_dir: &Path,
        project: &Project,
        garment: GarmentId,
        name: &str,
        kind: PatternKind,
        replace: bool,
    ) -> Result<LibraryPattern, WorkspaceError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CommandError::invalid("name", Invalid::Empty).into());
        }
        let g = project
            .garments()
            .get(&garment)
            .ok_or_else(|| CommandError::not_found(garment))?;
        if !project.panels().values().any(|p| p.garment == garment) {
            return Err(CommandError::invalid("garment", Invalid::Empty).into());
        }
        let existing = self.get(name).ok();
        if existing.is_some() && !replace {
            return Err(WorkspaceError::InLibrary {
                name: name.to_owned(),
            });
        }
        // The garment alone.
        let mut s = Session::new(project.clone());
        let others: Vec<GarmentId> = project
            .garments()
            .keys()
            .copied()
            .filter(|id| *id != garment)
            .collect();
        for other in others {
            s.execute(&Command::GarmentRemove(GarmentRemove {
                garment: other,
                confirm: true,
            }))?;
        }
        s.run_step(&Step::from_value(
            json!({"type": "project.rename", "name": name}),
        )?)?;

        fs::create_dir_all(&self.dir).map_err(|e| write_err(&self.dir, e))?;
        // Written aside, then put in place: a failed save leaves the
        // library as it was.
        let tmp = self.dir.join(format!(".saving-{}.tmp", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let written = (|| {
            project_dir::create(&tmp, s.project())?;
            if let Some(PrintSource::Image(file)) = g.print.as_ref().map(|p| &p.source) {
                let from = project_dir.join(file);
                let to = tmp.join(file);
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|e| write_err(parent, e))?;
                }
                fs::copy(&from, &to).map_err(|e| WorkspaceError::Read {
                    path: from.display().to_string(),
                    detail: e.to_string(),
                })?;
            }
            let meta = serde_json::to_vec_pretty(&Meta { kind }).map_err(|e| FileError::Json {
                detail: e.to_string(),
            })?;
            fs::write(tmp.join(META_FILE), meta).map_err(|e| write_err(&tmp, e))?;
            Ok::<(), WorkspaceError>(())
        })();
        if let Err(e) = written {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
        if let Some(old) = existing {
            projects::delete(&old.path)?;
        }
        let target = projects::free_path(&self.dir, name);
        fs::rename(&tmp, &target).map_err(|e| write_err(&target, e))?;
        Self::read(&target).ok_or_else(|| WorkspaceError::NotInLibrary {
            name: name.to_owned(),
        })
    }

    /// Rename the pattern `name` to `to`.
    pub fn rename(&self, name: &str, to: &str) -> Result<LibraryPattern, WorkspaceError> {
        let pattern = self.get(name)?;
        let to = to.trim();
        if to.is_empty() {
            return Err(CommandError::invalid("to", Invalid::Empty).into());
        }
        if self.get(to).is_ok_and(|other| other.path != pattern.path) {
            return Err(WorkspaceError::InLibrary {
                name: to.to_owned(),
            });
        }
        let dir = projects::rename(&pattern.path, to)?;
        Self::read(&dir).ok_or_else(|| WorkspaceError::NotInLibrary {
            name: to.to_owned(),
        })
    }

    /// Delete the pattern `name` (the projects it was added to keep their
    /// garments).
    pub fn delete(&self, name: &str) -> Result<(), WorkspaceError> {
        projects::delete(&self.get(name)?.path)
    }

    /// Pack the pattern `name` into the zip archive `out`, to share it.
    pub fn export(&self, name: &str, out: &Path) -> Result<(), WorkspaceError> {
        projects::pack(&self.get(name)?.path, out)
    }

    /// Add the pattern packed in the zip archive `archive` (by
    /// [`Self::export`], or any packed project of one garment); it takes
    /// a numbered name when its own is taken.
    pub fn import(&self, archive: &Path) -> Result<LibraryPattern, WorkspaceError> {
        fs::create_dir_all(&self.dir).map_err(|e| write_err(&self.dir, e))?;
        let taken: Vec<String> = self.list().into_iter().map(|p| p.name).collect();
        let dir = projects::unpack(archive, &self.dir)?;
        let refuse = |e: WorkspaceError| {
            let _ = fs::remove_dir_all(&dir);
            Err(e)
        };
        let Some(added) = Self::read(&dir) else {
            return refuse(FileError::NotAProject.into());
        };
        let project = match project_dir::load(&dir) {
            Ok(l) => l.project,
            Err(e) => return refuse(e.into()),
        };
        if project.panels().is_empty() {
            return refuse(CommandError::invalid("archive", Invalid::Empty).into());
        }
        let lower = |n: &str| n.to_lowercase();
        if !taken.iter().any(|t| lower(t) == lower(&added.name)) {
            return Ok(added);
        }
        let free = (2..)
            .map(|k| format!("{} {k}", added.name))
            .find(|n| !taken.iter().any(|t| lower(t) == lower(n)))
            .unwrap_or_else(|| added.name.clone());
        let dir = projects::rename(&dir, &free)?;
        Self::read(&dir).ok_or(WorkspaceError::NotInLibrary { name: free })
    }
}

/// The user's library folder when none is given: the desktop app's
/// support folder, where it keeps its body presets and motions (the
/// first of the platform's places that exists, else the usual one).
pub fn default_dir() -> Option<PathBuf> {
    let places = support_dirs();
    places
        .iter()
        .find(|p| p.is_dir())
        .or_else(|| places.last())
        .cloned()
}

/// Where the desktop app keeps its support folder, by platform.
pub fn support_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        if let Some(home) = &home {
            // Sandboxed builds live in their container.
            found.push(home.join(
                "Library/Containers/dev.rustle3d.rustle3d/Data/Library/Application Support/dev.rustle3d.rustle3d",
            ));
            found.push(home.join("Library/Application Support/dev.rustle3d.rustle3d"));
        }
    } else if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            found.push(PathBuf::from(appdata).join("dev.rustle3d").join("rustle3d"));
        }
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
        if let Some(data) = data {
            found.push(data.join("dev.rustle3d.rustle3d"));
        }
    }
    found
}

// The library through the command layer: actions, since they read and
// write the user's library folder.

/// The built-in patterns and the user's own.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryList {}

impl Described for LibraryList {
    const SUMMARY: &'static str = "The pattern library: `builtin`, the built-in patterns (each a template: its id, English title and kind; add one with template.apply), and `patterns`, the user's own saved with library.save (name, kind, folder, and the template it was drafted from if any; add one with library.add). Kinds: tops, bottoms, dresses_and_skirts, accessories.";
}

impl ActionSpec for LibraryList {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let builtin: Vec<Value> = Template::ALL
            .into_iter()
            .map(|t| json!({"template": t, "title": t.title(), "kind": t.kind()}))
            .collect();
        let patterns = ws.patterns()?.list();
        Ok(json!({"builtin": builtin, "patterns": patterns}))
    }
}

/// Add a pattern of the user's library to the project.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryAdd {
    /// The pattern's name in the library.
    pub name: String,
}

impl Described for LibraryAdd {
    const SUMMARY: &'static str = "Add a pattern of the user's library (see library.list) to the project as a new garment, worn over the others, drafted again for the avatar (pattern.add: pieces drafted from a template, a Seamly2D/Valentina pattern or formulas follow the body, the others come as drawn; graded in a size range). Its print image is copied into textures/. One undo step. Returns the garment, then the panels and seams.";
}

impl ActionSpec for LibraryAdd {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        self.run_with(ws, &Progress::default())
    }

    fn run_with(&self, ws: &mut Workspace, progress: &Progress) -> Result<Value, WorkspaceError> {
        let (mut doc, dir) = ws.patterns()?.document(&self.name)?;
        // Print images come into the project; the pattern names its copy.
        if let Some(garments) = doc
            .pointer_mut("/project/garments")
            .and_then(Value::as_object_mut)
        {
            for g in garments.values_mut() {
                let Some(image) = g.pointer_mut("/print/source/image") else {
                    continue;
                };
                let Some(file) = image.as_str().map(str::to_owned) else {
                    continue;
                };
                let from = dir.join(&file);
                if from.is_file() {
                    *image = Value::String(ws.adopt(&from, "textures")?);
                }
            }
        }
        let cmd = Command::PatternAdd(PatternAdd { pattern: doc });
        let out = ws.session_mut().execute_with(&cmd, progress)?;
        Ok(json!({ "created": out.created }))
    }
}

/// Save a garment to the user's library.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibrarySave {
    pub garment: GarmentId,
    /// Its name in the library.
    pub name: String,
    /// What it is, to sort the library.
    #[serde(default)]
    pub kind: PatternKind,
    /// Replace the library's pattern of that name, if there is one.
    #[serde(default)]
    pub replace: bool,
}

impl Described for LibrarySave {
    const SUMMARY: &'static str = "Save a garment to the user's pattern library, shared by every project, under a name and a kind (tops, bottoms, dresses_and_skirts, accessories; default tops): its pieces, seams, notches, internal lines, construction details, trims, constraints, fabric, colour, print, notes and grading, with the body it is drafted for. A name already in the library is refused unless `replace`. Returns the pattern as library.list lists it.";
}

impl ActionSpec for LibrarySave {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let saved = ws.patterns()?.save(
            ws.dir(),
            ws.session().project(),
            self.garment,
            &self.name,
            self.kind,
            self.replace,
        )?;
        Ok(json!(saved))
    }
}

/// Rename a pattern of the user's library.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryRename {
    pub name: String,
    pub to: String,
}

impl Described for LibraryRename {
    const SUMMARY: &'static str =
        "Rename a pattern of the user's library. Returns it as library.list lists it.";
}

impl ActionSpec for LibraryRename {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        Ok(json!(ws.patterns()?.rename(&self.name, &self.to)?))
    }
}

/// Delete a pattern of the user's library.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryDelete {
    pub name: String,
    /// Must be `true`: it cannot be undone.
    #[serde(default)]
    pub confirm: bool,
}

impl Described for LibraryDelete {
    const SUMMARY: &'static str = "Delete a pattern of the user's library (projects it was added to keep their garments). Cannot be undone: requires \"confirm\": true.";
}

impl ActionSpec for LibraryDelete {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        if !self.confirm {
            return Err(CommandError::ConfirmationRequired {
                command: "library.delete",
            }
            .into());
        }
        ws.patterns()?.delete(&self.name)?;
        Ok(json!({}))
    }
}

/// Pack a pattern of the user's library into a zip file, to share it.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryExport {
    pub name: String,
    /// Output path relative to the project folder. Defaults to
    /// `exports/<name>.zip`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Described for LibraryExport {
    const SUMMARY: &'static str = "Pack a pattern of the user's library into a zip file in the project folder (default exports/<name>.zip), to share it; library.import adds it to a library. Returns the written path, relative to the project folder.";
}

impl ActionSpec for LibraryExport {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let library = ws.patterns()?;
        let pattern = library.get(&self.name)?;
        let path = match &self.path {
            Some(p) => p.clone(),
            None => {
                let folder = projects::free_path(Path::new(""), &pattern.name);
                let stem = folder.file_stem().map(|s| s.to_string_lossy().into_owned());
                format!("exports/{}.zip", stem.unwrap_or_else(|| "pattern".into()))
            }
        };
        let out = ws.confined(&path)?;
        library.export(&pattern.name, &out)?;
        Ok(json!({ "path": path }))
    }
}

/// Add a pattern packed with library.export to the user's library.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryImport {
    /// The zip file, relative to the project folder.
    pub path: String,
}

impl Described for LibraryImport {
    const SUMMARY: &'static str = "Add a pattern packed with library.export (a zip file in the project folder) to the user's library; it takes a numbered name when its own is taken. Returns it as library.list lists it.";
}

impl ActionSpec for LibraryImport {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        // Checked to be inside the project folder first.
        ws.read(&self.path)?;
        let archive = ws.dir().join(&self.path);
        Ok(json!(ws.patterns()?.import(&archive)?))
    }
}
