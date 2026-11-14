//! `r3d`: headless Rustle3D command-line interface.
//!
//! Planned subcommands: `run <script>`, `simulate`, `export`, `render`,
//! `mcp` (MCP server over stdio). See `docs/specs/10-cli.md`.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("r3d {} (skeleton)", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}
