//! Environments that light photos (spec 35): the built-in set, shipped
//! inside the application, and the `.hdr` files of a project's
//! `environments/` folder. Provenance: `assets/environments/README.md`.

use std::fs;
use std::path::Path;

use r3d_io::hdr;
pub use r3d_io::hdr::{Hdr, texel_rgb};

use crate::CommandError;
use crate::workspace::{Workspace, WorkspaceError};

/// The built-in environments by name: photo studios from Poly Haven
/// (CC0), 1024 × 512.
pub const BUILTIN: [(&str, &[u8]); 5] = [
    (
        "ferndale",
        include_bytes!("../../../assets/environments/ferndale_studio_04_1k.hdr"),
    ),
    (
        "kontrast",
        include_bytes!("../../../assets/environments/studio_kontrast_04_1k.hdr"),
    ),
    (
        "neon",
        include_bytes!("../../../assets/environments/neon_photostudio_1k.hdr"),
    ),
    (
        "pav",
        include_bytes!("../../../assets/environments/pav_studio_02_1k.hdr"),
    ),
    (
        "wooden",
        include_bytes!("../../../assets/environments/wooden_studio_19_1k.hdr"),
    ),
];

/// The project's folder for environment files.
const FOLDER: &str = "environments";

/// The built-in environments' names.
pub fn builtin_names() -> [&'static str; 5] {
    BUILTIN.map(|(name, _)| name)
}

impl Workspace {
    /// The environment called `name`: a built-in one, or a `.hdr` file
    /// in the project folder (a path with a `/` or a `.`).
    pub fn environment(&self, name: &str) -> Result<Hdr, WorkspaceError> {
        if !name.contains(['/', '\\', '.']) {
            let bytes = BUILTIN
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, bytes)| *bytes)
                .ok_or_else(|| CommandError::Malformed {
                    name: "environment".into(),
                    detail: format!(
                        "expected one of: {}, or a .hdr file in the project folder",
                        builtin_names().join(", ")
                    ),
                })?;
            return Ok(hdr::decode(bytes)?);
        }
        Ok(hdr::decode(&self.read(name)?)?)
    }

    /// The `.hdr` files in the project's `environments/` folder, by path
    /// in the project, sorted.
    pub fn environment_files(&self) -> Vec<String> {
        let mut out: Vec<String> = fs::read_dir(self.dir().join(FOLDER))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                    .filter(|n| n.to_ascii_lowercase().ends_with(".hdr"))
                    .map(|n| format!("{FOLDER}/{n}"))
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Copy a `.hdr` picture from anywhere (one the user picked) into
    /// the project's `environments/` folder, as [`Self::adopt`] does,
    /// after checking that it can be read. Returns its relative path.
    pub fn adopt_environment(&self, source: &Path) -> Result<String, WorkspaceError> {
        let bytes = fs::read(source).map_err(|e| WorkspaceError::Read {
            path: source.display().to_string(),
            detail: e.to_string(),
        })?;
        hdr::decode(&bytes)?;
        self.adopt(source, FOLDER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_environments_are_readable_studios() {
        for (name, bytes) in BUILTIN {
            let h = hdr::decode(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!((h.width, h.height), (1024, 512), "{name}");
        }
    }

    #[test]
    fn environments_by_name_or_file() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::create(&dir.path().join("p.r3d"), "p").unwrap();
        assert_eq!(ws.environment("neon").unwrap().width, 1024);
        assert!(matches!(
            ws.environment("sunset"),
            Err(WorkspaceError::Command(CommandError::Malformed { .. }))
        ));
        assert!(ws.environment("environments/none.hdr").is_err());
        assert!(ws.environment("../x.hdr").is_err());

        // A tiny picture picked from elsewhere is copied in and listed.
        let source = dir.path().join("room.hdr");
        let mut bytes = b"#?RADIANCE\n\n-Y 1 +X 2\n".to_vec();
        bytes.extend([128, 128, 128, 129, 64, 64, 64, 129]);
        fs::write(&source, &bytes).unwrap();
        let path = ws.adopt_environment(&source).unwrap();
        assert_eq!(path, "environments/room.hdr");
        assert_eq!(ws.environment_files(), vec![path.clone()]);
        assert_eq!(ws.environment(&path).unwrap().width, 2);

        // Anything else is refused and not copied.
        let bad = dir.path().join("photo.hdr");
        fs::write(&bad, b"\x89PNG").unwrap();
        assert!(matches!(
            ws.adopt_environment(&bad),
            Err(WorkspaceError::Hdr(_))
        ));
        assert_eq!(ws.environment_files().len(), 1);
    }
}
