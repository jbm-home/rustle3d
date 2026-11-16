//! The user's own libraries, shared by every project: body presets. They
//! live in a folder the UI chooses (the app's support folder).

use std::path::Path;

use flutter_rust_bridge::frb;
use r3d_avatar::presets::{BodyPreset, PresetLibrary};
use r3d_core::Body;

use super::engine::ApiError;

fn presets_file(library_dir: &str) -> std::path::PathBuf {
    Path::new(library_dir).join("body-presets.json")
}

fn load(library_dir: &str) -> Result<PresetLibrary, ApiError> {
    PresetLibrary::load(&presets_file(library_dir))
        .map_err(|e| ApiError::new("library", e.to_string()))
}

fn save(library_dir: &str, lib: &PresetLibrary) -> Result<(), ApiError> {
    lib.save(&presets_file(library_dir))
        .map_err(|e| ApiError::new("library", e.to_string()))
}

/// Body presets as JSON: `[{name, body, builtin}]`, standard sizes first,
/// then the user's own from `library_dir`.
#[frb(sync)]
pub fn body_presets(library_dir: String) -> Result<String, ApiError> {
    let tag = |p: BodyPreset, builtin: bool| serde_json::json!({ "name": p.name, "body": p.body, "builtin": builtin });
    let mut all: Vec<serde_json::Value> = r3d_avatar::presets::builtin()
        .into_iter()
        .map(|p| tag(p, true))
        .collect();
    all.extend(
        load(&library_dir)?
            .presets
            .into_iter()
            .map(|p| tag(p, false)),
    );
    Ok(serde_json::Value::Array(all).to_string())
}

/// Save (or replace) one of the user's body presets. `body_json` is a
/// `Body` object as in the `avatar.set` command.
#[frb(sync)]
pub fn save_body_preset(
    library_dir: String,
    name: String,
    body_json: String,
) -> Result<(), ApiError> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(ApiError::new("invalid.empty", "the preset needs a name"));
    }
    let body: Body =
        serde_json::from_str(&body_json).map_err(|e| ApiError::new("malformed", e.to_string()))?;
    let mut lib = load(&library_dir)?;
    lib.upsert(BodyPreset { name, body });
    save(&library_dir, &lib)
}

/// Delete one of the user's body presets.
#[frb(sync)]
pub fn delete_body_preset(library_dir: String, name: String) -> Result<(), ApiError> {
    let mut lib = load(&library_dir)?;
    if lib.remove(&name) {
        save(&library_dir, &lib)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_presets_are_listed_saved_and_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib").to_string_lossy().into_owned();
        let list = || -> Vec<serde_json::Value> {
            serde_json::from_str(&body_presets(lib.clone()).unwrap()).unwrap()
        };
        let builtin = list().len();
        assert!(builtin >= 9);
        let body = r#"{"masculinity":0,"height_cm":165,"bust_cm":88,"waist_cm":70,"hips_cm":96}"#;
        save_body_preset(lib.clone(), "Me".into(), body.into()).unwrap();
        let all = list();
        assert_eq!(all.len(), builtin + 1);
        assert_eq!(all.last().unwrap()["builtin"], false);
        let blank = save_body_preset(lib.clone(), " ".into(), body.into());
        assert_eq!(blank.err().unwrap().code, "invalid.empty");
        delete_body_preset(lib.clone(), "Me".into()).unwrap();
        assert_eq!(list().len(), builtin);
    }
}
