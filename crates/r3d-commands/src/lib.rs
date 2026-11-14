//! The single command layer of Rustle3D.
//!
//! Every user-visible operation is a typed, serializable command with
//! validation, a JSON schema, and undo/redo through recorded changes. The
//! desktop app, the CLI and the MCP server all go through this crate; no
//! business logic lives anywhere else.
//!
//! See `docs/architecture.md` ("Command layer") and `docs/commands.md`.
