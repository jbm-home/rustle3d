//! Project folders on disk.
//!
//! ```text
//! MySkirt.r3d/
//!  ├─ project.json   the document
//!  ├─ avatars/       imported avatar meshes
//!  ├─ exports/       default output location
//!  └─ .r3d-cache/    derived data, safe to delete
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::file::{self, FileError, LoadedProject};
use crate::model::Project;

pub const PROJECT_FILE: &str = "project.json";
pub const SUBDIRS: [&str; 2] = ["avatars", "exports"];
const TMP_FILE: &str = ".project.json.tmp";

/// Accept either the project folder or its `project.json`.
pub fn resolve(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|n| n == PROJECT_FILE) && path.is_file() {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    }
}

/// Create a new project folder. Fails if `dir` exists and is not empty.
pub fn create(dir: &Path, project: &Project) -> Result<(), FileError> {
    if dir.exists() {
        let mut entries = fs::read_dir(dir).map_err(|e| FileError::io(dir, e))?;
        if entries.next().is_some() {
            return Err(FileError::AlreadyExists {
                path: dir.to_path_buf(),
            });
        }
    }
    for sub in SUBDIRS {
        let p = dir.join(sub);
        fs::create_dir_all(&p).map_err(|e| FileError::io(p, e))?;
    }
    save(dir, project)
}

/// Save atomically: write a temporary file, then rename it over
/// `project.json`, so a crash never leaves a truncated document.
pub fn save(dir: &Path, project: &Project) -> Result<(), FileError> {
    let json = file::to_json(project)?;
    let tmp = dir.join(TMP_FILE);
    fs::write(&tmp, json).map_err(|e| FileError::io(&tmp, e))?;
    let target = dir.join(PROJECT_FILE);
    fs::rename(&tmp, &target).map_err(|e| FileError::io(target, e))
}

/// Load (and migrate) the project in `dir`.
pub fn load(dir: &Path) -> Result<LoadedProject, FileError> {
    let path = resolve(dir).join(PROJECT_FILE);
    let text = fs::read_to_string(&path).map_err(|e| FileError::io(&path, e))?;
    file::from_json(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_save_load() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Skirt.r3d");
        let p = Project::new("Skirt");
        create(&dir, &p).unwrap();
        assert!(dir.join("avatars").is_dir());
        assert!(dir.join("exports").is_dir());
        assert_eq!(load(&dir).unwrap().project, p);
        assert_eq!(load(&dir.join(PROJECT_FILE)).unwrap().project, p);
        assert!(matches!(
            create(&dir, &p),
            Err(FileError::AlreadyExists { .. })
        ));
    }

    #[test]
    fn missing_project_is_an_error_not_a_panic() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(load(tmp.path()), Err(FileError::Io { .. })));
    }
}
