//! `rustle3d`: the Rustle3D desktop application (GUI stack to be chosen in M3).
//!
//! The UI only renders state and emits commands; see `docs/architecture.md`.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("rustle3d {} (skeleton)", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}
