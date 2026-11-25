//! The user's own libraries, shared by every project: body presets,
//! captured motions and patterns. They live in a folder the UI chooses
//! (the app's support folder).

use std::path::Path;

use flutter_rust_bridge::frb;
use r3d_avatar::motion_library::MotionLibrary;
use r3d_avatar::presets::{BodyPreset, PresetLibrary};
use r3d_commands::library::PatternLibrary;
use r3d_core::Body;

use super::engine::{ApiError, err};

/// The motion library in `library_dir` (its `motions` folder).
pub(crate) fn motions(library_dir: &str) -> MotionLibrary {
    MotionLibrary::new(&Path::new(library_dir).join("motions"))
}

/// The captured motions in the user's library, by name, sorted.
#[frb(sync)]
pub fn library_motions(library_dir: String) -> Vec<String> {
    motions(&library_dir).names()
}

/// Add the BVH file at `path` to the motion library, after checking that
/// the avatar can play it. Returns its name there.
#[frb(sync)]
pub fn add_library_motion(library_dir: String, path: String) -> Result<String, ApiError> {
    motions(&library_dir).add(Path::new(&path)).map_err(err)
}

/// Rename a motion of the library; returns its new name.
#[frb(sync)]
pub fn rename_library_motion(
    library_dir: String,
    name: String,
    to: String,
) -> Result<String, ApiError> {
    motions(&library_dir).rename(&name, &to).map_err(err)
}

/// Remove a motion from the library (projects keep their copies).
#[frb(sync)]
pub fn delete_library_motion(library_dir: String, name: String) -> Result<(), ApiError> {
    motions(&library_dir).remove(&name).map_err(err)
}

fn patterns(library_dir: &str) -> PatternLibrary {
    PatternLibrary::new(Path::new(library_dir))
}

/// The user's saved patterns as JSON: `[{name, kind, path, template?}]`,
/// by name (spec 32). Added to a project with the `library.add` step.
#[frb(sync)]
pub fn library_patterns(library_dir: String) -> Result<String, ApiError> {
    serde_json::to_string(&patterns(&library_dir).list())
        .map_err(|e| ApiError::new("internal", e.to_string()))
}

/// Rename a saved pattern; returns its new name.
#[frb(sync)]
pub fn rename_library_pattern(
    library_dir: String,
    name: String,
    to: String,
) -> Result<String, ApiError> {
    Ok(patterns(&library_dir).rename(&name, &to).map_err(err)?.name)
}

/// Delete a saved pattern (projects keep the garments added from it).
#[frb(sync)]
pub fn delete_library_pattern(library_dir: String, name: String) -> Result<(), ApiError> {
    patterns(&library_dir).delete(&name).map_err(err)
}

/// Pack a saved pattern into the zip file `archive`, to share it.
pub fn export_library_pattern(
    library_dir: String,
    name: String,
    archive: String,
) -> Result<(), ApiError> {
    patterns(&library_dir)
        .export(&name, Path::new(&archive))
        .map_err(err)
}

/// Add the pattern packed in the zip file `archive` to the library;
/// returns its name there.
pub fn import_library_pattern(library_dir: String, archive: String) -> Result<String, ApiError> {
    Ok(patterns(&library_dir)
        .import(Path::new(&archive))
        .map_err(err)?
        .name)
}

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

    #[test]
    fn library_motions_are_added_renamed_used_and_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib").to_string_lossy().into_owned();
        let walk =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../r3d-commands/tests/data/walk.bvh");
        let walk = walk.to_string_lossy().into_owned();
        assert!(library_motions(lib.clone()).is_empty());
        assert_eq!(add_library_motion(lib.clone(), walk).unwrap(), "walk");
        assert_eq!(
            rename_library_motion(lib.clone(), "walk".into(), "Stroll".into()).unwrap(),
            "Stroll"
        );
        assert_eq!(library_motions(lib.clone()), ["Stroll"]);
        // A project plays a copy of it.
        let engine =
            super::super::engine::Engine::open(tmp.path().join("P.r3d").to_string_lossy().into())
                .unwrap();
        let used = engine
            .use_library_motion(lib.clone(), "Stroll".into())
            .unwrap();
        assert_eq!(used, "motions/Stroll.bvh");
        assert_eq!(engine.motions().unwrap(), [used]);
        delete_library_motion(lib.clone(), "Stroll".into()).unwrap();
        assert!(library_motions(lib.clone()).is_empty());
        assert_eq!(engine.motions().unwrap().len(), 1);
        let missing = delete_library_motion(lib, "Stroll".into());
        assert_eq!(missing.err().unwrap().code, "library.missing");
    }
}
