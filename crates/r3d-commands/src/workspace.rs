//! A session bound to a project folder: saving, and actions that write
//! files (exports). Every file it writes stays inside the project folder.

use std::fs;
use std::path::{Component, Path, PathBuf};

use r3d_core::file::FileError;
use r3d_core::{Localizable, Message, Project, project_dir};
use serde_json::Value;

use crate::error::CommandError;
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
        match step {
            Step::Action(action) => action.run(self),
            other => Ok(self.session.run_step(other)?),
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

    /// Copy a file from anywhere (one the user picked) into the project's
    /// `imports/` folder, so the project keeps what it was made from.
    /// Returns its path relative to the project; a different file of the
    /// same name gets a numbered name, the same file is not copied twice.
    pub fn adopt(&self, source: &Path) -> Result<String, WorkspaceError> {
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
                format!("imports/{stem}{ext}")
            } else {
                format!("imports/{stem} ({k}){ext}")
            };
            match fs::read(self.dir.join(&candidate)) {
                Ok(existing) if existing == bytes => return Ok(candidate),
                Ok(_) => continue,
                Err(_) => return self.write(&candidate, &bytes),
            }
        }
        Err(WorkspaceError::Write {
            path: source.display().to_string(),
            detail: "no free name in imports/".into(),
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
        assert_eq!(ws.adopt(&src).unwrap(), "imports/skirt.svg");
        assert_eq!(ws.adopt(&src).unwrap(), "imports/skirt.svg");
        std::fs::write(&src, "b").unwrap();
        assert_eq!(ws.adopt(&src).unwrap(), "imports/skirt (2).svg");
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
