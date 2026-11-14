//! The single command layer of Rustle3D.
//!
//! Every user-visible operation is a typed, serializable command with
//! validation, a JSON schema, and undo/redo through recorded changes. The
//! desktop app, the CLI and the MCP server all go through this crate; no
//! business logic lives anywhere else.
//!
//! - [`Command`]: document edits (undoable), see [`commands`].
//! - [`Query`]: read-only questions, see [`queries`].
//! - [`Step`]: one entry of a script — a command, a query, undo/redo or an
//!   atomic batch.
//! - [`Session`]: a project plus its history; executes all of the above.
//!
//! See `docs/architecture.md` §4 and the generated `docs/commands.md`.

pub mod commands;
pub mod docs;
pub mod error;
pub mod history;
pub mod queries;
pub mod registry;
pub mod session;
pub mod spec;
pub mod step;

pub use error::{CommandError, Invalid};
pub use registry::{Command, Query};
pub use session::Session;
pub use spec::{CommandOutput, CommandSpec, Info, QuerySpec};
pub use step::Step;
