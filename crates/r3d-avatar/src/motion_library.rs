//! The user's motion library: captured motions (BVH files) kept in a
//! folder of their own, shared by every project. A motion is named by
//! its file's stem; a project plays a copy of it (spec 22).

use std::fs;
use std::path::{Path, PathBuf};

use r3d_core::{Localizable, Message};

use crate::captured::{CapturedMotion, MotionError};

/// The longest name a motion may have (characters).
const MAX_NAME: usize = 80;

/// Why the library could not do something.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MotionLibraryError {
    #[error("the motion library cannot be read or written: {detail}")]
    Io { detail: String },
    #[error("\"{name}\" cannot name a motion")]
    Name { name: String },
    #[error("the library already has a motion named \"{name}\"")]
    Exists { name: String },
    #[error("the library has no motion named \"{name}\"")]
    Missing { name: String },
    #[error(transparent)]
    Motion(#[from] MotionError),
}

impl Localizable for MotionLibraryError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Io { detail } => Message::new("library.io", text).arg("detail", detail),
            Self::Name { name } => Message::new("library.name", text).arg("name", name),
            Self::Exists { name } => Message::new("library.exists", text).arg("name", name),
            Self::Missing { name } => Message::new("library.missing", text).arg("name", name),
            Self::Motion(e) => e.message(),
        }
    }
}

fn io(e: std::io::Error) -> MotionLibraryError {
    MotionLibraryError::Io {
        detail: e.to_string(),
    }
}

/// `name` trimmed, if it can name a file on every system: not empty, not
/// too long, no path separator or character Windows refuses, not
/// starting with a dot nor ending with one.
fn valid(name: &str) -> Result<String, MotionLibraryError> {
    let name = name.trim();
    let bad = name.is_empty()
        || name.chars().count() > MAX_NAME
        || name.starts_with('.')
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c));
    if bad {
        Err(MotionLibraryError::Name {
            name: name.to_owned(),
        })
    } else {
        Ok(name.to_owned())
    }
}

/// The motion library in a folder.
#[derive(Debug, Clone)]
pub struct MotionLibrary {
    dir: PathBuf,
}

impl MotionLibrary {
    /// The library in `dir` (created when the first motion is added).
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
        }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.bvh"))
    }

    /// The motions' names, sorted (case-insensitively).
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                    .filter_map(|n| {
                        let stem = n.strip_suffix(".bvh").or_else(|| n.strip_suffix(".BVH"))?;
                        valid(stem).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();
        names.sort_by_key(|n| n.to_lowercase());
        names
    }

    /// The file of the motion `name`.
    pub fn path(&self, name: &str) -> Result<PathBuf, MotionLibraryError> {
        let file = self.file(&valid(name)?);
        if file.is_file() {
            Ok(file)
        } else {
            Err(MotionLibraryError::Missing {
                name: name.to_owned(),
            })
        }
    }

    /// Add the BVH file at `source`, after checking that the avatar can
    /// play it; named after the file (numbered if another motion has that
    /// name; the same file again is not copied twice). Returns its name.
    pub fn add(&self, source: &Path) -> Result<String, MotionLibraryError> {
        let bytes = fs::read(source).map_err(io)?;
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let stem = valid(stem).unwrap_or_else(|_| "motion".to_owned());
        CapturedMotion::from_bvh(&stem, &String::from_utf8_lossy(&bytes))?;
        fs::create_dir_all(&self.dir).map_err(io)?;
        for k in 1..=1000 {
            let name = if k == 1 {
                stem.clone()
            } else {
                format!("{stem} ({k})")
            };
            match fs::read(self.file(&name)) {
                Ok(existing) if existing == bytes => return Ok(name),
                Ok(_) => continue,
                Err(_) => {
                    fs::write(self.file(&name), &bytes).map_err(io)?;
                    return Ok(name);
                }
            }
        }
        Err(MotionLibraryError::Exists { name: stem })
    }

    /// Rename the motion `name` to `to`; returns the new name (trimmed).
    pub fn rename(&self, name: &str, to: &str) -> Result<String, MotionLibraryError> {
        let from = self.path(name)?;
        let to = valid(to)?;
        if to == name {
            return Ok(to);
        }
        // A different file, not the same one in another case.
        let target = self.file(&to);
        if target.exists() && !to.eq_ignore_ascii_case(name) {
            return Err(MotionLibraryError::Exists { name: to });
        }
        fs::rename(from, target).map_err(io)?;
        Ok(to)
    }

    /// Remove the motion `name` from the library.
    pub fn remove(&self, name: &str) -> Result<(), MotionLibraryError> {
        fs::remove_file(self.path(name)?).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn walk() -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../r3d-commands/tests/data/walk.bvh"),
        )
        .unwrap()
    }

    #[test]
    fn motions_are_added_renamed_and_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = MotionLibrary::new(&tmp.path().join("motions"));
        assert!(lib.names().is_empty());
        let source = tmp.path().join("slow walk.bvh");
        std::fs::write(&source, walk()).unwrap();
        assert_eq!(lib.add(&source).unwrap(), "slow walk");
        // The same file again is the same motion; another one of that
        // name is numbered.
        assert_eq!(lib.add(&source).unwrap(), "slow walk");
        std::fs::write(&source, walk() + "\n").unwrap();
        assert_eq!(lib.add(&source).unwrap(), "slow walk (2)");
        assert_eq!(lib.names(), ["slow walk", "slow walk (2)"]);

        assert_eq!(lib.rename("slow walk (2)", " Stroll ").unwrap(), "Stroll");
        assert_eq!(lib.names(), ["slow walk", "Stroll"]);
        assert_eq!(
            lib.rename("Stroll", "slow walk"),
            Err(MotionLibraryError::Exists {
                name: "slow walk".into()
            })
        );
        for bad in ["", "  ", "a/b", "..", ".hidden", "x:y", "end."] {
            assert!(
                matches!(
                    lib.rename("Stroll", bad),
                    Err(MotionLibraryError::Name { .. })
                ),
                "{bad}"
            );
        }
        assert!(lib.path("Stroll").unwrap().is_file());

        lib.remove("Stroll").unwrap();
        assert_eq!(lib.names(), ["slow walk"]);
        assert_eq!(
            lib.remove("Stroll"),
            Err(MotionLibraryError::Missing {
                name: "Stroll".into()
            })
        );
        assert!(matches!(
            lib.path("../x"),
            Err(MotionLibraryError::Name { .. })
        ));
    }

    #[test]
    fn a_file_the_avatar_cannot_play_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = MotionLibrary::new(&tmp.path().join("motions"));
        let source = tmp.path().join("bad.bvh");
        std::fs::write(&source, "HIERARCHY").unwrap();
        assert!(matches!(
            lib.add(&source),
            Err(MotionLibraryError::Motion(MotionError::Format { .. }))
        ));
        assert!(lib.names().is_empty());
    }
}
