//! `r3d`: headless Rustle3D command-line interface.
//!
//! Output is JSON on stdout; errors are JSON on stderr
//! (`{"error": {"code", "args", "text"}}`). Exit codes: 0 success,
//! 1 user error (bad input, file problem), 2 internal error.
//! See `docs/specs/10-cli.md`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use r3d_commands::step::parse_query;
use r3d_commands::{Command, CommandError, Query, Session, Step};
use r3d_core::{Localizable, Message, Project, project_dir};
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "r3d",
    version,
    about = "Headless Rustle3D: create projects, run command scripts, query them."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new, empty project folder (e.g. `Skirt.r3d`).
    New {
        path: PathBuf,
        /// Project name (defaults to the folder name without `.r3d`).
        #[arg(long)]
        name: Option<String>,
    },
    /// Run a script (JSON array of steps) on a project, then save it.
    /// Nothing is saved if any step fails.
    Run {
        project: PathBuf,
        /// Script file, or `-` for stdin.
        script: PathBuf,
        /// Execute and print results, but do not save.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run one query (JSON object) and print its result.
    Query {
        project: PathBuf,
        /// e.g. '{"type": "project.summary"}'
        query: String,
    },
    /// List every operation, or print the JSON schema of one.
    Schema { name: Option<String> },
}

/// A failure with its exit code.
struct Failure {
    message: Message,
    code: u8,
}

impl<E: Localizable> From<E> for Failure {
    fn from(e: E) -> Self {
        let message = e.message();
        let code = if message.code == "internal" { 2 } else { 1 };
        Failure { message, code }
    }
}

fn print_json(value: &Value) -> Result<(), Failure> {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    writeln!(out, "{text}").map_err(|e| Failure {
        message: Message::new("io.stdout", e.to_string()),
        code: 2,
    })
}

fn read_script(path: &Path) -> Result<String, Failure> {
    let io_err = |e: std::io::Error| Failure {
        message: Message::new("io.read", format!("cannot read {}: {e}", path.display()))
            .arg("path", path.display()),
        code: 1,
    };
    if path.as_os_str() == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(io_err)?;
        Ok(s)
    } else {
        std::fs::read_to_string(path).map_err(io_err)
    }
}

fn default_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().trim_end_matches(".r3d").to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Untitled".to_owned())
}

fn cmd_new(path: &Path, name: Option<String>) -> Result<(), Failure> {
    let project = Project::new(name.unwrap_or_else(|| default_name(path)));
    project_dir::create(path, &project)?;
    print_json(&json!({ "created": path.display().to_string(), "name": project.name() }))
}

fn cmd_run(project: &Path, script: &Path, dry_run: bool) -> Result<(), Failure> {
    let steps = Step::parse_script(&read_script(script)?)?;
    let dir = project_dir::resolve(project);
    let mut session = Session::new(project_dir::load(&dir)?.project);
    let mut results = Vec::with_capacity(steps.len());
    for (i, step) in steps.iter().enumerate() {
        let result = session.run_step(step).map_err(|e| CommandError::AtStep {
            index: i,
            source: Box::new(e),
        })?;
        results.push(json!({ "step": i, "result": result }));
    }
    let save = !dry_run && session.is_dirty();
    if save {
        project_dir::save(&dir, session.project())?;
    }
    print_json(&json!({ "results": results, "saved": save }))
}

fn cmd_query(project: &Path, query: &str) -> Result<(), Failure> {
    let value: Value = serde_json::from_str(query).map_err(|e| CommandError::Malformed {
        name: "query".into(),
        detail: e.to_string(),
    })?;
    let query = parse_query(value)?;
    let loaded = project_dir::load(project)?;
    print_json(&query.run(&loaded.project)?)
}

fn cmd_schema(name: Option<&str>) -> Result<(), Failure> {
    let commands = Command::registry();
    let queries = Query::registry();
    match name {
        None => {
            let list = |infos: &[r3d_commands::Info]| -> Vec<Value> {
                infos
                    .iter()
                    .map(|i| json!({ "name": i.name, "summary": i.summary, "destructive": i.destructive }))
                    .collect()
            };
            print_json(&json!({ "commands": list(&commands), "queries": list(&queries) }))
        }
        Some(n) => {
            let info = commands
                .iter()
                .chain(queries.iter())
                .find(|i| i.name == n)
                .ok_or_else(|| CommandError::UnknownType { name: n.to_owned() })?;
            print_json(&(info.schema)())
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::New { path, name } => cmd_new(&path, name),
        Cmd::Run {
            project,
            script,
            dry_run,
        } => cmd_run(&project, &script, dry_run),
        Cmd::Query { project, query } => cmd_query(&project, &query),
        Cmd::Schema { name } => cmd_schema(name.as_deref()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(f) => {
            let text = serde_json::to_string_pretty(&json!({ "error": f.message }))
                .unwrap_or_else(|_| f.message.text.clone());
            eprintln!("{text}");
            ExitCode::from(f.code)
        }
    }
}
