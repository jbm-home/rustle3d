//! MCP server for Rustle3D.
//!
//! Exposes the command layer as MCP tools generated from the command schemas
//! (one source of truth). Served over stdio by the CLI (headless mode) or over
//! a local socket by the running desktop app (attached mode).
//!
//! See `docs/specs/09-mcp-connector.md`.
