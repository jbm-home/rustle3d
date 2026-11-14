//! Versioned JSON serialization of a project.
//!
//! File layout:
//! ```json
//! { "format": "rustle3d.project", "schema_version": 1, "project": { ... } }
//! ```
//! Loading migrates older versions step by step on the raw JSON value
//! (`MIGRATIONS[v - 1]` turns version `v` into `v + 1`), then deserializes
//! into the typed model and checks integrity. Newer files are refused.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::integrity::IntegrityError;
use crate::message::{Localizable, Message};
use crate::model::Project;

/// Identifies a Rustle3D project file.
pub const FORMAT: &str = "rustle3d.project";
/// Version written by this build.
pub const SCHEMA_VERSION: u32 = 2;

type Migration = fn(Value) -> Result<Value, FileError>;

/// `MIGRATIONS[i]` migrates schema version `i + 1` to `i + 2`.
const MIGRATIONS: &[Migration] = &[migrate_v1_to_v2];

fn object<'a>(
    value: &'a mut Value,
    what: &str,
) -> Result<&'a mut serde_json::Map<String, Value>, FileError> {
    value.as_object_mut().ok_or_else(|| FileError::Json {
        detail: format!("{what} is not an object"),
    })
}

/// v2 adds notches, internal lines, seam allowances and mirror links.
fn migrate_v1_to_v2(mut project: Value) -> Result<Value, FileError> {
    let root = object(&mut project, "project")?;
    root.insert("notches".into(), Value::Object(Default::default()));
    root.insert("internal_lines".into(), Value::Object(Default::default()));
    if let Some(panels) = root.get_mut("panels") {
        for panel in object(panels, "panels")?.values_mut() {
            let panel = object(panel, "panel")?;
            panel.insert("seam_allowance_mm".into(), Value::from(0.0));
            panel.insert("mirror_of".into(), Value::Null);
        }
    }
    Ok(project)
}

// Every version below the current one must have a migration.
const _: () = assert!(MIGRATIONS.len() as u32 + 1 == SCHEMA_VERSION);

/// Errors while reading or writing project files.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid project JSON: {detail}")]
    Json { detail: String },
    #[error("not a Rustle3D project file")]
    NotAProject,
    #[error("unsupported project schema version {found}")]
    UnsupportedVersion { found: u64 },
    #[error(
        "project schema version {found} is newer than this build supports ({supported}); please update Rustle3D"
    )]
    TooNew { found: u64, supported: u32 },
    #[error("corrupted project: {0}")]
    Corrupted(#[from] IntegrityError),
    #[error("{path} already exists and is not empty")]
    AlreadyExists { path: PathBuf },
}

impl FileError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        FileError::Io {
            path: path.into(),
            source,
        }
    }
}

impl Localizable for FileError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Io { path, source } => Message::new("file.io", text)
                .arg("path", path.display())
                .arg("detail", source),
            Self::Json { detail } => Message::new("file.json", text).arg("detail", detail),
            Self::NotAProject => Message::new("file.not_a_project", text),
            Self::UnsupportedVersion { found } => {
                Message::new("file.unsupported_version", text).arg("found", found)
            }
            Self::TooNew { found, supported } => Message::new("file.too_new", text)
                .arg("found", found)
                .arg("supported", supported),
            Self::Corrupted(e) => {
                let inner = e.message();
                let mut m = Message::new("file.corrupted", text).arg("reason", inner.code);
                m.args.extend(inner.args);
                m
            }
            Self::AlreadyExists { path } => {
                Message::new("file.already_exists", text).arg("path", path.display())
            }
        }
    }
}

/// A loaded project, with the version it was migrated from (if any).
#[derive(Debug)]
pub struct LoadedProject {
    pub project: Project,
    pub migrated_from: Option<u32>,
}

#[derive(Serialize)]
struct Envelope<'a> {
    format: &'static str,
    schema_version: u32,
    project: &'a Project,
}

/// Serialize to pretty JSON with a stable key order and a trailing newline.
pub fn to_json(project: &Project) -> Result<String, FileError> {
    let envelope = Envelope {
        format: FORMAT,
        schema_version: SCHEMA_VERSION,
        project,
    };
    let mut s = serde_json::to_string_pretty(&envelope).map_err(|e| FileError::Json {
        detail: e.to_string(),
    })?;
    s.push('\n');
    Ok(s)
}

/// Parse, migrate and validate a project file.
pub fn from_json(text: &str) -> Result<LoadedProject, FileError> {
    let json_err = |e: serde_json::Error| FileError::Json {
        detail: e.to_string(),
    };
    let mut root: Value = serde_json::from_str(text).map_err(json_err)?;
    let obj = root.as_object_mut().ok_or(FileError::NotAProject)?;
    if obj.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(FileError::NotAProject);
    }
    let found = obj
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(FileError::NotAProject)?;
    if found > u64::from(SCHEMA_VERSION) {
        return Err(FileError::TooNew {
            found,
            supported: SCHEMA_VERSION,
        });
    }
    if found == 0 {
        return Err(FileError::UnsupportedVersion { found });
    }
    let mut project = obj.remove("project").ok_or(FileError::NotAProject)?;
    let mut version = found as u32;
    while version < SCHEMA_VERSION {
        let migrate = MIGRATIONS
            .get(version as usize - 1)
            .ok_or(FileError::UnsupportedVersion { found })?;
        project = migrate(project)?;
        version += 1;
    }
    let project: Project = serde_json::from_value(project).map_err(json_err)?;
    project.check_integrity()?;
    Ok(LoadedProject {
        project,
        migrated_from: (found < u64::from(SCHEMA_VERSION)).then_some(found as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_new_project() {
        let p = Project::new("Skirt");
        let json = to_json(&p).unwrap();
        let back = from_json(&json).unwrap();
        assert_eq!(back.project, p);
        assert_eq!(back.migrated_from, None);
        assert_eq!(to_json(&back.project).unwrap(), json);
    }

    #[test]
    fn refuses_foreign_and_future_files() {
        assert!(matches!(from_json("{}"), Err(FileError::NotAProject)));
        assert!(matches!(from_json("[1,2]"), Err(FileError::NotAProject)));
        assert!(matches!(from_json("not json"), Err(FileError::Json { .. })));
        let future = r#"{"format":"rustle3d.project","schema_version":999,"project":{}}"#;
        assert!(matches!(
            from_json(future),
            Err(FileError::TooNew { found: 999, .. })
        ));
        let zero = r#"{"format":"rustle3d.project","schema_version":0,"project":{}}"#;
        assert!(matches!(
            from_json(zero),
            Err(FileError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn refuses_corrupted_project() {
        let p = Project::new("x");
        let json = to_json(&p)
            .unwrap()
            .replace("\"next_id\": 2", "\"next_id\": 1");
        assert!(matches!(from_json(&json), Err(FileError::Corrupted(_))));
    }

    #[test]
    fn error_messages_are_structured() {
        let m = FileError::TooNew {
            found: 9,
            supported: 1,
        }
        .message();
        assert_eq!(m.code, "file.too_new");
        assert_eq!(m.args.get("found").map(String::as_str), Some("9"));
    }
}
