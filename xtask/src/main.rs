//! Developer tasks: `cargo xtask <task>`.
//!
//! - `gen-docs`: regenerate `docs/commands.md` from the command registry.

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
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("gen-docs") => gen_docs(),
        _ => Err("usage: cargo xtask gen-docs".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
