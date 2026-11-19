//! Developer tasks: `cargo xtask <task>`.
//!
//! - `gen-docs`: regenerate `docs/commands.md` from the command registry.
//! - `fetch-makehuman`: rebuild the avatar body pack from MakeHuman's CC0
//!   assets (network).
//! - `run [--release] [-- <flutter args>]`: launch the desktop app for this
//!   OS in debug (default) or release.
//! - `build [--release] [-- <flutter args>]`: build the CLI and the desktop
//!   app for this OS.
//! - `version [YY.M.D]`: set the version to a release date (today's by
//!   default) in `Cargo.toml` and `app/pubspec.yaml`.
//! - `make-help`: print the `Makefile`'s header and its targets (`make`).

mod dev;
mod makehuman;
mod version;

use std::path::PathBuf;
use std::process::ExitCode;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn gen_docs() -> Result<(), String> {
    let path = root().join("docs/commands.md");
    let mut md = r3d_commands::docs::commands_markdown();
    md.push('\n');
    std::fs::write(&path, md).map_err(|e| format!("{}: {e}", path.display()))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (task, rest) = match args.split_first() {
        Some((task, rest)) => (Some(task.as_str()), rest),
        None => (None, &[][..]),
    };
    let result = match task {
        Some("gen-docs") => gen_docs(),
        Some("fetch-makehuman") => makehuman::run(&root()),
        Some("run") => dev::run(&root(), dev::Task::Run, rest),
        Some("build") => dev::run(&root(), dev::Task::Build, rest),
        Some("make-help") => dev::make_help(&root()),
        Some("version") => version::run(&root(), rest),
        _ => Err(
            "usage: cargo xtask <gen-docs|fetch-makehuman|run|build|version|make-help> [--release] [-- <flutter args>]"
                .to_owned(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
