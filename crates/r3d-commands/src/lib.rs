//! The single command layer of Rustle3D.
//!
//! Every user-visible operation is a typed, serializable command with
//! validation, a JSON schema, and undo/redo through recorded changes. The
//! desktop app, the CLI and the MCP server all go through this crate; no
//! business logic lives anywhere else.
//!
//! - [`Command`]: document edits (undoable), see [`commands`].
//! - [`Query`]: read-only questions, see [`queries`].
//! - [`Action`]: operations that write files (exports), see [`actions`];
//!   they run on a [`Workspace`] (a session bound to a project folder).
//! - [`Step`]: one entry of a script — a command, a query, undo/redo or an
//!   atomic batch.
//! - [`Session`]: a project plus its history; executes all of the above.
//! - [`projects`]: whole project folders: rename, delete, zip archives.
//! - [`production`]: indicators per piece and the marker (M30).
//!
//! See `docs/architecture.md` §4 and the generated `docs/commands.md`.

pub mod actions;
pub mod commands;
pub mod docs;
pub mod drape;
pub mod error;
pub mod history;
pub mod production;
pub mod progress;
pub mod projects;
pub mod queries;
pub mod registry;
pub mod script;
pub mod session;
pub mod spec;
pub mod step;
pub mod templates;
pub mod workspace;

pub use error::{CommandError, Invalid};
pub use progress::{Progress, Status};
pub use registry::{Action, Command, Query};
pub use script::{Script, ScriptError};
pub use session::Session;
pub use spec::{CommandOutput, CommandSpec, Info, QuerySpec};
pub use step::Step;
pub use workspace::{Workspace, WorkspaceError};
