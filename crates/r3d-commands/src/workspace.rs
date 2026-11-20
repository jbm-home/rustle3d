//! A session bound to a project folder: saving, and actions that write
//! files (exports). Every file it writes stays inside the project folder.

use std::fs;
use std::path::{Component, Path, PathBuf};

use r3d_core::file::FileError;
use r3d_core::{Localizable, Message, Project, project_dir};
use serde_json::Value;

use crate::error::CommandError;
use crate::progress::Progress;
use crate::session::Session;
use crate::step::Step;

/// Errors of workspace operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error(transparent)]
    File(#[from] FileError),
    #[error("`{path}` is outside the project folder; use a relative path without `..`")]
    OutsideProject { path: String },
    #[error("cannot write {path}: {detail}")]
    Write { path: String, detail: String },
    #[error("cannot read {path}: {detail}")]
    Read { path: String, detail: String },
    #[error(transparent)]
    Import(#[from] r3d_io::import::ImportError),
    #[error(transparent)]
    Image(#[from] r3d_io::techpack::TechPackError),
    #[error(transparent)]
    Texture(#[from] r3d_io::texture::TextureError),
    #[error(transparent)]
    Model(#[from] r3d_io::model::ModelError),
    #[error("not a readable zip archive: {detail}")]
    Archive { detail: String },
    #[error(
        "the archive holds no Rustle3D project (one project.json, at the top or in one folder)"
    )]
    NotAProjectArchive,
    #[error("the archive unpacks to more than {limit_mb} MB")]
    ArchiveTooLarge { limit_mb: u64 },
}

impl Localizable for WorkspaceError {
    fn message(&self) -> Message {
        match self {
            Self::Command(e) => e.message(),
            Self::File(e) => e.message(),
            Self::OutsideProject { path } => {
                Message::new("outside_project", self.to_string()).arg("path", path)
            }
            Self::Write { path, detail } => Message::new("write_failed", self.to_string())
                .arg("path", path)
                .arg("detail", detail),
            Self::Read { path, detail } => Message::new("read_failed", self.to_string())
                .arg("path", path)
                .arg("detail", detail),
            Self::Import(e) => e.message(),
            Self::Image(e) => e.message(),
            Self::Texture(e) => e.message(),
            Self::Model(e) => e.message(),
            Self::Archive { detail } => {
                Message::new("archive.invalid", self.to_string()).arg("detail", detail)
            }
            Self::NotAProjectArchive => Message::new("archive.no_project", self.to_string()),
            Self::ArchiveTooLarge { limit_mb } => {
                Message::new("archive.too_large", self.to_string()).arg("limit_mb", limit_mb)
            }
        }
    }
}

/// A project folder opened for editing.
#[derive(Debug)]
pub struct Workspace {
    dir: PathBuf,
    session: Session,
    /// Set when the file was migrated from an older schema and the original
    /// has not been backed up yet.
    pending_backup: Option<u32>,
}

impl Workspace {
    /// Create a new project folder.
    pub fn create(dir: &Path, name: &str) -> Result<Self, WorkspaceError> {
        let project = Project::new(name);
        project_dir::create(dir, &project)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            session: Session::new(project),
            pending_backup: None,
        })
    }

    /// Open an existing project folder (or its `project.json`).
    pub fn open(path: &Path) -> Result<Self, WorkspaceError> {
        let dir = project_dir::resolve(path);
        let loaded = project_dir::load(&dir)?;
        crate::bodies::load(&dir, &loaded.project);
        Ok(Self {
            dir,
            session: Session::new(loaded.project),
            pending_backup: loaded.migrated_from,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    /// End the current merge run of the history (a gesture is over).
    pub fn seal_history(&mut self) {
        self.session.seal_history();
    }

    pub(crate) fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Save the project. The first save after a migration keeps the
    /// original file as `project.v{N}.json.bak`.
    pub fn save(&mut self) -> Result<(), WorkspaceError> {
        if let Some(version) = self.pending_backup {
            let from = self.dir.join(project_dir::PROJECT_FILE);
            let to = self.dir.join(format!("project.v{version}.json.bak"));
            if !to.exists() {
                fs::copy(&from, &to).map_err(|e| WorkspaceError::Write {
                    path: to.display().to_string(),
                    detail: e.to_string(),
                })?;
            }
            self.pending_backup = None;
        }
        project_dir::save(&self.dir, self.session.project())?;
        self.session.mark_saved();
        Ok(())
    }

    /// Run any step, including actions that write files.
    pub fn run_step(&mut self, step: &Step) -> Result<Value, WorkspaceError> {
        self.run_step_with(step, &Progress::default())
    }

    /// [`Workspace::run_step`], reporting to `progress` and stopping when
    /// it asks (long steps only; see [`Progress`]).
    pub fn run_step_with(
        &mut self,
        step: &Step,
        progress: &Progress,
    ) -> Result<Value, WorkspaceError> {
        match step {
            Step::Action(action) => action.run(self, progress),
            other => Ok(self.session.run_step_with(other, progress)?),
        }
    }

    /// Resolve a path given by a caller, refusing anything that could leave
    /// the project folder (absolute paths, `..`, symlinks pointing out).
    pub(crate) fn confined(&self, relative: &str) -> Result<PathBuf, WorkspaceError> {
        let outside = || WorkspaceError::OutsideProject {
            path: relative.to_owned(),
        };
        let rel = Path::new(relative);
        if relative.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err(outside());
        }
        let canon = |p: &Path| {
            p.canonicalize().map_err(|e| WorkspaceError::Write {
                path: relative.to_owned(),
                detail: e.to_string(),
            })
        };
        let root = canon(&self.dir)?;
        let path = self.dir.join(rel);
        let parent = path.parent().ok_or_else(outside)?;
        // Check the deepest existing ancestor *before* creating anything, so
        // a symlinked directory cannot make us create folders outside.
        let mut existing = parent;
        while fs::symlink_metadata(existing).is_err() {
            existing = existing.parent().ok_or_else(outside)?;
        }
        if !canon(existing)?.starts_with(&root) {
            return Err(outside());
        }
        fs::create_dir_all(parent).map_err(|e| WorkspaceError::Write {
            path: relative.to_owned(),
            detail: e.to_string(),
        })?;
        if !canon(parent)?.starts_with(&root) {
            return Err(outside());
        }
        // Writing through a symlink would modify its target, wherever it is.
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(outside());
        }
        Ok(path)
    }

    /// Read a file inside the project folder.
    pub(crate) fn read(&self, relative: &str) -> Result<Vec<u8>, WorkspaceError> {
        let failed = |e: std::io::Error| WorkspaceError::Read {
            path: relative.to_owned(),
            detail: e.to_string(),
        };
        let rel = Path::new(relative);
        if relative.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err(WorkspaceError::OutsideProject {
                path: relative.to_owned(),
            });
        }
        let root = self.dir.canonicalize().map_err(failed)?;
        let path = self.dir.join(rel).canonicalize().map_err(failed)?;
        if !path.starts_with(&root) {
            return Err(WorkspaceError::OutsideProject {
                path: relative.to_owned(),
            });
        }
        fs::read(&path).map_err(failed)
    }

    /// Copy a print image from anywhere (one the user picked) into the
    /// project's `textures/` folder, as [`Self::adopt`] does, after
    /// checking that it is a PNG or JPEG that can be read (spec 25).
    pub fn adopt_texture(&self, source: &Path) -> Result<String, WorkspaceError> {
        let bytes = fs::read(source).map_err(|e| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        })?;
        r3d_io::texture::decode(&bytes)?;
        self.adopt(source, "textures")
    }

    /// Copy a body model from anywhere (one the user picked) into the
    /// project's `avatars/` folder, as [`Self::adopt`] does, after
    /// checking that it can be read (spec 27). Returns its relative path.
    pub fn adopt_body(&self, source: &Path) -> Result<String, WorkspaceError> {
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let format = r3d_io::model::ModelFormat::of_name(name)
            .ok_or_else(|| CommandError::invalid("path", crate::error::Invalid::Format))?;
        let bytes = fs::read(source).map_err(|e| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        })?;
        crate::bodies::register(&bytes, format)?;
        self.adopt(source, "avatars")
    }

    /// The captured motion in the BVH file at `relative` in the project
    /// folder.
    pub fn captured_motion(
        &self,
        relative: &str,
    ) -> Result<r3d_avatar::motion::Motion, WorkspaceError> {
        Ok(crate::drape::captured_motion(
            relative,
            &self.read(relative)?,
        )?)
    }

    /// Copy a BVH motion capture from anywhere into the project's
    /// `motions/` folder, as [`Self::adopt`] does, after checking that the
    /// avatar can play it (spec 22). Returns its relative path.
    pub fn adopt_motion(&self, source: &Path) -> Result<String, WorkspaceError> {
        let bytes = fs::read(source).map_err(|e| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        })?;
        crate::drape::captured_motion(&source.display().to_string(), &bytes)?;
        self.adopt(source, "motions")
    }

    /// Copy a pattern file from anywhere into the project's `imports/`
    /// folder, as [`Self::adopt`] does; a Seamly2D or Valentina pattern
    /// brings the measurement file it names (at that path from the
    /// pattern, else by its name next to it), if it is there. Returns the
    /// pattern's relative path and the measurement file's.
    pub fn adopt_pattern(&self, source: &Path) -> Result<(String, Option<String>), WorkspaceError> {
        let pattern = self.adopt(source, "imports")?;
        let parametric = source
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("val") || e.eq_ignore_ascii_case("sm2d"));
        if !parametric {
            return Ok((pattern, None));
        }
        let bytes = fs::read(source).map_err(|e| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        })?;
        let Some(link) =
            r3d_io::import::val::linked_measurements(&bytes).map_err(CommandError::from)?
        else {
            return Ok((pattern, None));
        };
        let folder = source.parent().unwrap_or(Path::new(""));
        let by_name = Path::new(&link).file_name().map(|n| folder.join(n));
        let found = [Some(folder.join(&link)), by_name]
            .into_iter()
            .flatten()
            .find(|p| p.is_file());
        Ok(match found {
            Some(m) => (pattern, Some(self.adopt(&m, "imports")?)),
            None => (pattern, None),
        })
    }

    /// Copy a file from anywhere (one the user picked) into the project's
    /// `folder` (`imports` for patterns), so the project keeps what it was
    /// made from. Returns its path relative to the project; a different
    /// file of the same name gets a numbered name, the same file is not
    /// copied twice.
    pub fn adopt(&self, source: &Path, folder: &str) -> Result<String, WorkspaceError> {
        let failed = |e: std::io::Error| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        };
        let bytes = fs::read(source).map_err(failed)?;
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.starts_with('.'))
            .unwrap_or("pattern");
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
            _ => (name, String::new()),
        };
        for k in 1.. {
            let candidate = if k == 1 {
                format!("{folder}/{stem}{ext}")
            } else {
                format!("{folder}/{stem} ({k}){ext}")
            };
            match fs::read(self.dir.join(&candidate)) {
                Ok(existing) if existing == bytes => return Ok(candidate),
                Ok(_) => continue,
                Err(_) => return self.write(&candidate, &bytes),
            }
        }
        Err(WorkspaceError::Write {
            path: source.display().to_string(),
            detail: format!("no free name in {folder}/"),
        })
    }

    /// Write a file inside the project folder; returns its relative path.
    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) -> Result<String, WorkspaceError> {
        let path = self.confined(relative)?;
        fs::write(&path, bytes).map_err(|e| WorkspaceError::Write {
            path: relative.to_owned(),
            detail: e.to_string(),
        })?;
        Ok(relative.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_confined_to_the_project() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap();
        assert!(ws.confined("exports/a.svg").is_ok());
        assert!(ws.confined("sub/dir/b.pdf").is_ok());
        for bad in [
            "",
            "../x.svg",
            "exports/../../x.svg",
            "/tmp/x.svg",
            "./x.svg",
        ] {
            assert!(
                matches!(ws.confined(bad), Err(WorkspaceError::OutsideProject { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn imports_are_adopted_and_read_back() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap();
        let src = tmp.path().join("skirt.svg");
        std::fs::write(&src, "a").unwrap();
        assert_eq!(ws.adopt(&src, "imports").unwrap(), "imports/skirt.svg");
        assert_eq!(ws.adopt(&src, "imports").unwrap(), "imports/skirt.svg");
        std::fs::write(&src, "b").unwrap();
        assert_eq!(ws.adopt(&src, "imports").unwrap(), "imports/skirt (2).svg");
        assert_eq!(ws.read("imports/skirt (2).svg").unwrap(), b"b");
        std::fs::write(tmp.path().join("secret"), "x").unwrap();
        for bad in ["../secret", "/etc/passwd", ""] {
            assert!(matches!(
                ws.read(bad),
                Err(WorkspaceError::OutsideProject { .. })
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, ws.dir().join("escape")).unwrap();
        assert!(matches!(
            ws.confined("escape/x.svg"),
            Err(WorkspaceError::OutsideProject { .. })
        ));
        // No folder may be created outside, even transiently.
        assert!(matches!(
            ws.confined("escape/new/dir/x.svg"),
            Err(WorkspaceError::OutsideProject { .. })
        ));
        assert!(!outside.join("new").exists());

        // A file symlink inside the project pointing outside is refused too.
        let victim = outside.join("victim.txt");
        std::fs::write(&victim, "keep").unwrap();
        std::os::unix::fs::symlink(&victim, ws.dir().join("exports/link.svg")).unwrap();
        assert!(matches!(
            ws.write("exports/link.svg", b"overwritten"),
            Err(WorkspaceError::OutsideProject { .. })
        ));
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");

        // Nor read through one.
        assert!(matches!(
            ws.read("exports/link.svg"),
            Err(WorkspaceError::OutsideProject { .. })
        ));
    }
}
