//! An MCP Bundle (`.mcpb`) for desktop assistants that install MCP servers
//! from a file (an open format: a zip holding `manifest.json`, see
//! <https://github.com/modelcontextprotocol/mcpb>, manifest version 0.3).
//!
//! The bundle holds no program: its server is `r3d mcp --attach` from the
//! `r3d` installed with the app, named by its full path, so it relays to
//! the project open in the app and follows the app's updates. It is made on
//! the machine that uses it, for that machine's system only.

use std::fs::File;
use std::io::Write as _;
use std::path::Path;

use r3d_core::{Localizable, Message};
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// The app's icon, shown by the assistant next to the extension.
const ICON: &[u8] =
    include_bytes!("../../../app/macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_256.png");

/// Why the bundle could not be written.
#[derive(Debug, thiserror::Error)]
#[error("cannot write the extension {path}: {detail}")]
pub struct BundleError {
    path: String,
    detail: String,
}

impl Localizable for BundleError {
    fn message(&self) -> Message {
        Message::new("bundle_write", self.to_string())
            .arg("path", &self.path)
            .arg("detail", &self.detail)
    }
}

/// The bundle's `manifest.json`: the server is `r3d` at `r3d`, attached.
pub fn manifest(r3d: &Path) -> Value {
    let platform = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let command = r3d.to_string_lossy();
    json!({
        "manifest_version": "0.3",
        "name": "rustle3d",
        "display_name": "Rustle3D",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Design garments in Rustle3D: draft and sew pattern pieces, drape them on the avatar and look at the result, live in the project open in the app.",
        "author": { "name": "Rustle3D", "url": "https://rustle3d.com" },
        "homepage": "https://rustle3d.com",
        "documentation": "https://rustle3d.com/en/guide/",
        "license": "GPL-3.0-or-later",
        "icon": "icon.png",
        "server": {
            "type": "binary",
            "entry_point": command,
            "mcp_config": {
                "command": command,
                "args": ["mcp", "--attach"],
            },
        },
        "compatibility": { "platforms": [platform] },
    })
}

/// Write the bundle for `r3d` to `out`.
pub fn write(r3d: &Path, out: &Path) -> Result<(), BundleError> {
    let fail = |e: &dyn std::fmt::Display| BundleError {
        path: out.display().to_string(),
        detail: e.to_string(),
    };
    let manifest = serde_json::to_vec_pretty(&manifest(r3d)).map_err(|e| fail(&e))?;
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(File::create(out).map_err(|e| fail(&e))?);
    for (name, bytes) in [("manifest.json", manifest.as_slice()), ("icon.png", ICON)] {
        zip.start_file(name, options).map_err(|e| fail(&e))?;
        zip.write_all(bytes).map_err(|e| fail(&e))?;
    }
    zip.finish().map_err(|e| fail(&e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use zip::ZipArchive;

    use super::*;

    #[test]
    fn the_bundle_runs_r3d_attached() {
        let tmp = tempfile::tempdir().unwrap();
        let r3d = tmp.path().join("My Apps").join("r3d");
        let out = tmp.path().join("rustle3d.mcpb");
        write(&r3d, &out).unwrap();
        let mut zip = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut text = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let m: Value = serde_json::from_str(&text).unwrap();
        let config = &m["server"]["mcp_config"];
        assert_eq!(config["command"], r3d.to_string_lossy().as_ref());
        assert_eq!(config["args"], json!(["mcp", "--attach"]));
        assert_eq!(m["server"]["type"], "binary");
        for field in ["manifest_version", "name", "version", "description"] {
            assert!(m[field].is_string(), "{field}");
        }
        assert!(m["author"]["name"].is_string());
        assert_eq!(m["compatibility"]["platforms"].as_array().unwrap().len(), 1);
        assert!(zip.by_name("icon.png").unwrap().size() > 0);
    }

    #[test]
    fn a_folder_that_is_not_there_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("missing").join("rustle3d.mcpb");
        let e = write(Path::new("r3d"), &out).unwrap_err();
        assert_eq!(e.message().code, "bundle_write");
    }
}
