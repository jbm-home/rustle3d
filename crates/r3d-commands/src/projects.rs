//! Whole project folders: rename, delete, and pack into or unpack from a
//! zip archive (to send a project to someone or keep a copy of it).
//!
//! Zip is an open format (PKWARE APPNOTE). An archive holds one project
//! folder as it is on disk, `Name.r3d/…`, without the disposable
//! `.r3d-cache/`.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use r3d_core::file::FileError;
use r3d_core::project_dir::{self, PROJECT_FILE, SUBDIRS};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::step::Step;
use crate::workspace::{Workspace, WorkspaceError};

/// Derived data, never packed nor unpacked.
const CACHE_DIR: &str = ".r3d-cache";

/// What an archive may unpack to at most: well above any project (avatar
/// meshes are a few MB), well below filling a disk.
pub const MAX_UNPACKED_BYTES: u64 = 2 << 30;
const MAX_ENTRIES: usize = 10_000;

/// A folder path in `parent` for a project called `name` that does not
/// exist yet: `Name.r3d`, then `Name 2.r3d`, …
pub fn free_path(parent: &Path, name: &str) -> PathBuf {
    folder_names(parent, name)
        .find(|p| !p.exists())
        .unwrap_or_else(|| parent.join("Untitled.r3d"))
}

/// The folder names a project called `name` can take, in order of
/// preference.
fn folder_names(parent: &Path, name: &str) -> impl Iterator<Item = PathBuf> {
    let stem = folder_stem(name);
    let parent = parent.to_path_buf();
    (1..).map(move |n| match n {
        1 => parent.join(format!("{stem}.r3d")),
        n => parent.join(format!("{stem} {n}.r3d")),
    })
}

/// `name` made safe as a folder name on every system.
fn folder_stem(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    // Windows drops trailing dots and spaces from folder names.
    let safe = safe.trim().trim_end_matches('.').trim();
    if safe.is_empty() {
        "Untitled".into()
    } else {
        safe.into()
    }
}

/// Rename the project in `dir` to `name` (through `project.rename`), and
/// its folder to match when that name is free. Returns the folder's path.
pub fn rename(dir: &Path, name: &str) -> Result<PathBuf, WorkspaceError> {
    let mut ws = Workspace::open(dir)?;
    let step = Step::from_value(json!({ "type": "project.rename", "name": name }))?;
    ws.run_step(&step)?;
    ws.save()?;
    let dir = ws.dir().to_path_buf();
    drop(ws);

    let Some(parent) = dir.parent().map(Path::to_path_buf) else {
        return Ok(dir);
    };
    let lower = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_lowercase());
    // Stops at its own folder name too (case aside, for case-insensitive
    // systems): it is free for this project.
    let Some(target) = folder_names(&parent, name).find(|p| lower(p) == lower(&dir) || !p.exists())
    else {
        return Ok(dir);
    };
    if target.file_name() == dir.file_name() {
        return Ok(dir);
    }
    fs::rename(&dir, &target).map_err(|e| WorkspaceError::Write {
        path: target.display().to_string(),
        detail: e.to_string(),
    })?;
    Ok(target)
}

/// Delete the project folder `dir` and everything in it. Refuses a folder
/// without a `project.json`, so a wrong path never wipes other files.
pub fn delete(dir: &Path) -> Result<(), WorkspaceError> {
    let dir = project_dir::resolve(dir);
    if !dir.join(PROJECT_FILE).is_file() {
        return Err(FileError::NotAProject.into());
    }
    fs::remove_dir_all(&dir).map_err(|e| WorkspaceError::Write {
        path: dir.display().to_string(),
        detail: e.to_string(),
    })
}

/// Pack the project folder `dir` into the zip archive `out`.
pub fn pack(dir: &Path, out: &Path) -> Result<(), WorkspaceError> {
    let dir = project_dir::resolve(dir);
    if !dir.join(PROJECT_FILE).is_file() {
        return Err(FileError::NotAProject.into());
    }
    let root = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled.r3d".into());
    // Listed before the archive is created: saved inside the project, it
    // must not pack itself.
    let mut files = Vec::new();
    list(&dir, Path::new(""), &mut files)?;

    let write_err = |e: &dyn std::fmt::Display| WorkspaceError::Write {
        path: out.display().to_string(),
        detail: e.to_string(),
    };
    let mut zip = ZipWriter::new(File::create(out).map_err(|e| write_err(&e))?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.add_directory(format!("{root}/"), options)
        .map_err(|e| write_err(&e))?;
    for (rel, is_dir) in files {
        let name = format!("{root}/{}", archive_name(&rel));
        if is_dir {
            zip.add_directory(format!("{name}/"), options)
                .map_err(|e| write_err(&e))?;
            continue;
        }
        let path = dir.join(&rel);
        let mut file = File::open(&path).map_err(|e| WorkspaceError::Read {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;
        zip.start_file(name, options).map_err(|e| write_err(&e))?;
        io::copy(&mut file, &mut zip).map_err(|e| write_err(&e))?;
    }
    zip.finish().map_err(|e| write_err(&e))?;
    Ok(())
}

/// Every folder and file under `dir/rel` (paths relative to `dir`), except
/// the cache, temporary files and links.
fn list(dir: &Path, rel: &Path, out: &mut Vec<(PathBuf, bool)>) -> Result<(), WorkspaceError> {
    let here = dir.join(rel);
    let read_err = |e: io::Error| WorkspaceError::Read {
        path: here.display().to_string(),
        detail: e.to_string(),
    };
    let mut entries = fs::read_dir(&here)
        .map_err(read_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(read_err)?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name();
        if rel.as_os_str().is_empty() && name == CACHE_DIR {
            continue;
        }
        if name.to_string_lossy().ends_with(".tmp") {
            continue;
        }
        let kind = entry.file_type().map_err(read_err)?;
        let path = rel.join(&name);
        if kind.is_dir() {
            out.push((path.clone(), true));
            list(dir, &path, out)?;
        } else if kind.is_file() {
            out.push((path, false));
        }
    }
    Ok(())
}

/// A relative path as an archive entry name (always `/`-separated).
fn archive_name(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Unpack the project in the zip archive `archive` into a new folder in
/// `parent`, and return that folder. The project must open; otherwise
/// nothing is left behind.
pub fn unpack(archive: &Path, parent: &Path) -> Result<PathBuf, WorkspaceError> {
    let invalid = |detail: String| WorkspaceError::Archive { detail };
    let file = File::open(archive).map_err(|e| WorkspaceError::Read {
        path: archive.display().to_string(),
        detail: e.to_string(),
    })?;
    let mut zip = ZipArchive::new(file).map_err(|e| invalid(e.to_string()))?;
    if zip.len() > MAX_ENTRIES {
        return Err(WorkspaceError::ArchiveTooLarge {
            limit_mb: MAX_UNPACKED_BYTES >> 20,
        });
    }

    // The project is `project.json` at the top, or in one folder.
    let names = zip
        .file_names()
        .map(|n| n.map(|n| n.into_owned()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| invalid(e.to_string()))?;
    let mut roots = names.iter().filter_map(|n| {
        let mut parts = n.split('/');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(PROJECT_FILE), None, _) => Some(String::new()),
            (Some(folder), Some(PROJECT_FILE), None) => Some(format!("{folder}/")),
            _ => None,
        }
    });
    let (Some(prefix), None) = (roots.next(), roots.next()) else {
        return Err(WorkspaceError::NotAProjectArchive);
    };
    let stem = match prefix.trim_end_matches('/') {
        "" => archive.file_stem(),
        folder => Path::new(folder).file_stem(),
    }
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_default();

    let target = free_path(parent, &stem);
    let result = extract(&mut zip, &prefix, &target).and_then(|()| {
        for sub in SUBDIRS {
            let p = target.join(sub);
            fs::create_dir_all(&p).map_err(|e| WorkspaceError::Write {
                path: p.display().to_string(),
                detail: e.to_string(),
            })?;
        }
        project_dir::load(&target)?;
        Ok(())
    });
    match result {
        Ok(()) => Ok(target),
        Err(e) => {
            // Best effort: the error says what went wrong either way.
            let _ = fs::remove_dir_all(&target);
            Err(e)
        }
    }
}

/// Extract the entries under `prefix` into `target`, refusing any that
/// would land outside it and stopping past [`MAX_UNPACKED_BYTES`].
fn extract(zip: &mut ZipArchive<File>, prefix: &str, target: &Path) -> Result<(), WorkspaceError> {
    let too_large = || WorkspaceError::ArchiveTooLarge {
        limit_mb: MAX_UNPACKED_BYTES >> 20,
    };
    let write_err = |path: &Path, e: &dyn std::fmt::Display| WorkspaceError::Write {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    fs::create_dir_all(target).map_err(|e| write_err(target, &e))?;
    let mut budget = MAX_UNPACKED_BYTES;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| WorkspaceError::Archive {
            detail: e.to_string(),
        })?;
        let name = entry
            .name()
            .map_err(|e| WorkspaceError::Archive {
                detail: e.to_string(),
            })?
            .into_owned();
        let Some(rel) = name.strip_prefix(prefix).map(Path::new) else {
            continue;
        };
        if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err(WorkspaceError::Archive {
                detail: format!("unsafe entry `{name}`"),
            });
        }
        if rel.starts_with(CACHE_DIR) || entry.is_symlink() {
            continue;
        }
        let path = target.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| write_err(&path, &e))?;
            continue;
        }
        if entry.size() > budget {
            return Err(too_large());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| write_err(dir, &e))?;
        }
        let mut out = File::create(&path).map_err(|e| write_err(&path, &e))?;
        // The declared size can lie: count what actually comes out.
        let written = io::copy(&mut (&mut entry).take(budget + 1), &mut out).map_err(|e| {
            WorkspaceError::Archive {
                detail: e.to_string(),
            }
        })?;
        budget = budget.checked_sub(written).ok_or_else(too_large)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn project(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(format!("{name}.r3d"));
        Workspace::create(&dir, name).unwrap();
        fs::write(dir.join("avatars/body.obj"), "v 0 0 0\n").unwrap();
        fs::create_dir_all(dir.join(CACHE_DIR)).unwrap();
        fs::write(dir.join(CACHE_DIR).join("drape.bin"), "cache").unwrap();
        dir
    }

    fn name_of(dir: &Path) -> String {
        Workspace::open(dir)
            .unwrap()
            .session()
            .project()
            .name()
            .to_owned()
    }

    #[test]
    fn free_path_skips_taken_names_and_unsafe_characters() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(free_path(tmp.path(), "A/B?"), tmp.path().join("A B.r3d"));
        assert_eq!(
            free_path(tmp.path(), " .. "),
            tmp.path().join("Untitled.r3d")
        );
        project(tmp.path(), "Skirt");
        assert_eq!(
            free_path(tmp.path(), "Skirt"),
            tmp.path().join("Skirt 2.r3d")
        );
    }

    #[test]
    fn rename_changes_the_name_and_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        project(tmp.path(), "Dress");

        let renamed = rename(&dir, "Summer skirt").unwrap();
        assert_eq!(renamed, tmp.path().join("Summer skirt.r3d"));
        assert!(!dir.exists());
        assert_eq!(name_of(&renamed), "Summer skirt");

        // A folder of that name exists: the next free one.
        let again = rename(&renamed, "Dress").unwrap();
        assert_eq!(again, tmp.path().join("Dress 2.r3d"));
        assert_eq!(name_of(&again), "Dress");

        // Same folder name: stays where it is.
        assert_eq!(rename(&again, "Dress").unwrap(), again);
    }

    #[test]
    fn rename_refuses_an_empty_name() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        assert!(rename(&dir, "  ").is_err());
        assert_eq!(name_of(&dir), "Skirt");
    }

    #[test]
    fn delete_removes_a_project_but_nothing_else() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        delete(&dir).unwrap();
        assert!(!dir.exists());

        let other = tmp.path().join("photos");
        fs::create_dir_all(&other).unwrap();
        assert!(matches!(
            delete(&other),
            Err(WorkspaceError::File(FileError::NotAProject))
        ));
        assert!(other.exists());
    }

    #[test]
    fn pack_then_unpack_gives_the_same_project() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        let zip = tmp.path().join("skirt.zip");
        pack(&dir, &zip).unwrap();

        let names: Vec<String> = ZipArchive::new(File::open(&zip).unwrap())
            .unwrap()
            .file_names()
            .map(|n| n.unwrap().into_owned())
            .collect();
        assert!(names.contains(&"Skirt.r3d/project.json".to_owned()));
        assert!(names.contains(&"Skirt.r3d/avatars/body.obj".to_owned()));
        assert!(names.iter().all(|n| !n.contains(CACHE_DIR)));

        let into = tmp.path().join("elsewhere");
        fs::create_dir_all(&into).unwrap();
        let out = unpack(&zip, &into).unwrap();
        assert_eq!(out, into.join("Skirt.r3d"));
        assert_eq!(
            fs::read(out.join(PROJECT_FILE)).unwrap(),
            fs::read(dir.join(PROJECT_FILE)).unwrap()
        );
        assert_eq!(
            fs::read_to_string(out.join("avatars/body.obj")).unwrap(),
            "v 0 0 0\n"
        );
        assert!(out.join("exports").is_dir());
        // Twice: a second copy beside the first.
        assert_eq!(unpack(&zip, &into).unwrap(), into.join("Skirt 2.r3d"));
    }

    #[test]
    fn pack_into_the_project_folder_does_not_pack_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        let zip = dir.join("exports/Skirt.zip");
        pack(&dir, &zip).unwrap();
        let names: Vec<String> = ZipArchive::new(File::open(&zip).unwrap())
            .unwrap()
            .file_names()
            .map(|n| n.unwrap().into_owned())
            .collect();
        assert!(!names.iter().any(|n| n.ends_with(".zip")));
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for (name, data) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn unpack_accepts_a_project_at_the_top_of_the_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        let json = fs::read(dir.join(PROJECT_FILE)).unwrap();
        let zip = tmp.path().join("From a friend.zip");
        write_zip(&zip, &[(PROJECT_FILE, &json)]);
        let out = unpack(&zip, tmp.path()).unwrap();
        assert_eq!(out, tmp.path().join("From a friend.r3d"));
        assert_eq!(name_of(&out), "Skirt");
    }

    #[test]
    fn unpack_refuses_what_is_not_a_project_and_leaves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let into = tmp.path().join("projects");
        fs::create_dir_all(&into).unwrap();
        let empty = || fs::read_dir(&into).unwrap().next().is_none();

        let not_zip = tmp.path().join("notes.zip");
        fs::write(&not_zip, "hello").unwrap();
        assert!(matches!(
            unpack(&not_zip, &into),
            Err(WorkspaceError::Archive { .. })
        ));

        let photos = tmp.path().join("photos.zip");
        write_zip(&photos, &[("a.png", b"png")]);
        assert!(matches!(
            unpack(&photos, &into),
            Err(WorkspaceError::NotAProjectArchive)
        ));

        let broken = tmp.path().join("broken.zip");
        write_zip(&broken, &[("X.r3d/project.json", b"{ not json")]);
        assert!(matches!(
            unpack(&broken, &into),
            Err(WorkspaceError::File(_))
        ));
        assert!(empty());
    }

    #[test]
    fn unpack_refuses_entries_that_leave_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project(tmp.path(), "Skirt");
        let json = fs::read(dir.join(PROJECT_FILE)).unwrap();
        let into = tmp.path().join("projects");
        fs::create_dir_all(&into).unwrap();
        let evil = tmp.path().join("evil.zip");
        write_zip(
            &evil,
            &[
                ("X.r3d/project.json", &json),
                ("X.r3d/../../owned.txt", b"x"),
            ],
        );
        assert!(matches!(
            unpack(&evil, &into),
            Err(WorkspaceError::Archive { .. })
        ));
        assert!(!tmp.path().join("owned.txt").exists());
        assert!(!into.join("X.r3d").exists());
    }
}
