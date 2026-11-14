//! `rustle3d`: the Rustle3D desktop application (egui + wgpu).
//!
//! The UI only renders state and emits commands; see `docs/architecture.md`.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("rustle3d {} (skeleton)", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}
