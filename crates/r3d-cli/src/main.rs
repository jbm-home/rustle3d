//! `r3d`: headless Rustle3D command-line interface.
//!
//! Output is JSON on stdout; errors are JSON on stderr
//! (`{"error": {"code", "args", "text"}}`). Exit codes: 0 success,
//! 1 user error (bad input, file problem), 2 internal error.
//! See `docs/specs/10-cli.md`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use r3d_commands::step::parse_query;
use r3d_commands::{Action, Command, CommandError, Info, Query, Script, Session, Step, Workspace};
use r3d_core::{Localizable, Message};
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "r3d",
    version,
    about = "Headless Rustle3D: create projects, run command scripts, export patterns, serve MCP."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Svg,
    Pdf,
}

#[derive(Clone, Copy, ValueEnum)]
enum PaperArg {
    A4,
    Letter,
    Single,
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
    /// The project is not saved if any step fails (files already exported
    /// by earlier steps stay).
    Run {
        project: PathBuf,
        /// Script file, or `-` for stdin.
        script: PathBuf,
        /// Validate and print results without writing anything (actions
        /// such as exports are refused).
        #[arg(long)]
        dry_run: bool,
    },
    /// Run one query (JSON object) and print its result.
    Query {
        project: PathBuf,
        /// e.g. '{"type": "project.summary"}'
        query: String,
    },
    /// Export the pattern at 1:1 scale into the project folder.
    Export {
        project: PathBuf,
        #[arg(long, value_enum)]
        format: Format,
        /// PDF page layout.
        #[arg(long, value_enum, default_value = "a4")]
        paper: PaperArg,
        /// Output path relative to the project (default: exports/<name>.<ext>).
        #[arg(long)]
        out: Option<String>,
    },
    /// List every operation, or print the JSON schema of one.
    Schema { name: Option<String> },
    /// Serve the project to AI agents over MCP (stdio). The project is
    /// created if the folder does not exist. Changes are saved as they happen.
    Mcp {
        #[arg(long)]
        project: PathBuf,
    },
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

fn io_failure(code: &'static str, text: String) -> Failure {
    Failure {
        message: Message::new(code, text),
        code: 2,
    }
}

fn print_json(value: &Value) -> Result<(), Failure> {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    writeln!(out, "{text}").map_err(|e| io_failure("io.stdout", e.to_string()))
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
    let name = name.unwrap_or_else(|| default_name(path));
    Workspace::create(path, &name)?;
    print_json(&json!({ "created": path.display().to_string(), "name": name }))
}

fn print_results(results: Vec<Value>, saved: bool) -> Result<(), Failure> {
    let results: Vec<Value> = results
        .into_iter()
        .enumerate()
        .map(|(i, result)| json!({ "step": i, "result": result }))
        .collect();
    print_json(&json!({ "results": results, "saved": saved }))
}

fn cmd_run(project: &Path, script: &Path, dry_run: bool) -> Result<(), Failure> {
    let script = Script::parse(&read_script(script)?)?;
    let mut ws = Workspace::open(project)?;
    if dry_run {
        // A bare session: actions are refused, nothing touches the disk.
        let mut session = Session::new(ws.session().project().clone());
        let results = script.run(|step| session.run_step(step))?;
        return print_results(results, false);
    }
    let results = script.run(|step| ws.run_step(step))?;
    let save = ws.session().is_dirty();
    if save {
        ws.save()?;
    }
    print_results(results, save)
}

fn cmd_query(project: &Path, query: &str) -> Result<(), Failure> {
    let value: Value = serde_json::from_str(query).map_err(|e| CommandError::Malformed {
        name: "query".into(),
        detail: e.to_string(),
    })?;
    let query = parse_query(value)?;
    let ws = Workspace::open(project)?;
    print_json(&ws.session().query(&query)?)
}

fn cmd_export(
    project: &Path,
    format: Format,
    paper: PaperArg,
    out: Option<String>,
) -> Result<(), Failure> {
    let mut step = match format {
        Format::Svg => json!({ "type": "export.svg" }),
        Format::Pdf => json!({
            "type": "export.pdf",
            "paper": match paper {
                PaperArg::A4 => "a4",
                PaperArg::Letter => "letter",
                PaperArg::Single => "single",
            }
        }),
    };
    if let Some(path) = out {
        step["path"] = Value::from(path);
    }
    let mut ws = Workspace::open(project)?;
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

fn cmd_schema(name: Option<&str>) -> Result<(), Failure> {
    let groups = [
        ("commands", Command::registry()),
        ("queries", Query::registry()),
        ("actions", Action::registry()),
    ];
    match name {
        None => {
            let list = |infos: &[Info]| -> Vec<Value> {
                infos
                    .iter()
                    .map(|i| json!({ "name": i.name, "summary": i.summary, "destructive": i.destructive }))
                    .collect()
            };
            let mut out = serde_json::Map::new();
            for (key, infos) in &groups {
                out.insert((*key).into(), Value::from(list(infos)));
            }
            print_json(&Value::Object(out))
        }
        Some(n) => {
            let info = groups
                .iter()
                .flat_map(|(_, infos)| infos.iter())
                .find(|i| i.name == n)
                .ok_or_else(|| CommandError::UnknownType { name: n.to_owned() })?;
            print_json(&(info.schema)())
        }
    }
}

fn cmd_mcp(project: &Path) -> Result<(), Failure> {
    let ws = if project.exists() {
        Workspace::open(project)?
    } else {
        Workspace::create(project, &default_name(project))?
    };
    r3d_mcp::serve_stdio(ws).map_err(|e| io_failure("mcp", e.to_string()))
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
        Cmd::Export {
            project,
            format,
            paper,
            out,
        } => cmd_export(&project, format, paper, out),
        Cmd::Schema { name } => cmd_schema(name.as_deref()),
        Cmd::Mcp { project } => cmd_mcp(&project),
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
