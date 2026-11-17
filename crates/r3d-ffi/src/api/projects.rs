//! Whole project folders, from the start screen: a free folder name for a
//! new project, rename, delete, and zip archives (`r3d_commands::projects`).

use std::path::Path;

use flutter_rust_bridge::frb;
use r3d_commands::projects;

use super::engine::{ApiError, err};

/// A folder path in `parent` for a new project called `name`.
#[frb(sync)]
pub fn free_project_path(parent: String, name: String) -> String {
    projects::free_path(Path::new(&parent), &name)
        .to_string_lossy()
        .into_owned()
}

/// Rename the (closed) project at `path`; returns its folder, renamed to
/// match when that name is free.
#[frb(sync)]
pub fn rename_project(path: String, name: String) -> Result<String, ApiError> {
    projects::rename(Path::new(&path), &name)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(err)
}

/// Delete the (closed) project folder at `path` for good.
#[frb(sync)]
pub fn delete_project(path: String) -> Result<(), ApiError> {
    projects::delete(Path::new(&path)).map_err(err)
}

/// Pack the project at `path` into the zip archive `archive`.
pub fn export_project_zip(path: String, archive: String) -> Result<(), ApiError> {
    projects::pack(Path::new(&path), Path::new(&archive)).map_err(err)
}

/// Unpack the project in the zip archive `archive` into a new folder in
/// `parent`; returns that folder.
pub fn import_project_zip(archive: String, parent: String) -> Result<String, ApiError> {
    projects::unpack(Path::new(&archive), Path::new(&parent))
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(err)
}
