//! `cargo xtask run|build [--release] [-- <flutter args>]`: launch or build
//! the desktop app (and build the CLI) for this machine's OS, in debug or
//! release. Cargokit builds `r3d-ffi` in the same mode as the Flutter app.
//!
//! Ctrl+C goes to Flutter, which closes the app; this task waits for it
//! instead of dying first and giving the prompt back over Flutter's output.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What `run` and `build` do.
#[derive(Clone, Copy)]
pub enum Task {
    Run,
    Build,
}

/// The Flutter desktop target of this OS.
fn desktop() -> Result<&'static str, String> {
    match std::env::consts::OS {
        "windows" => Ok("windows"),
        "macos" => Ok("macos"),
        "linux" => Ok("linux"),
        other => Err(format!("no desktop app for {other}")),
    }
}

/// The Flutter tool.
#[cfg(not(windows))]
fn flutter() -> Result<Command, String> {
    Ok(Command::new("flutter"))
}

/// The Flutter tool. On Windows `flutter.bat` asks "Terminate batch job
/// (Y/N)?" after a Ctrl+C, over the shell's prompt; so the batch file only
/// brings Flutter's own files up to date (`--version`), and the tool runs
/// from its Dart snapshot, as the batch file would run it.
#[cfg(windows)]
fn flutter() -> Result<Command, String> {
    let bat = std::env::var_os("PATH")
        .iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join("flutter.bat"))
        .find(|bat| bat.is_file())
        .ok_or("flutter.bat is not in PATH (see docs/building.md)")?;
    let root: PathBuf = bat
        .parent()
        .and_then(Path::parent)
        .ok_or("flutter.bat is not in Flutter's bin folder")?
        .to_owned();
    let mut update = Command::new(&bat);
    update
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    status(update)?;
    let cache = root.join("bin").join("cache");
    let mut cmd = Command::new(cache.join("dart-sdk").join("bin").join("dart.exe"));
    cmd.env("FLUTTER_ROOT", &root)
        .arg(format!(
            "--packages={}",
            root.join("packages")
                .join("flutter_tools")
                .join(".dart_tool")
                .join("package_config.json")
                .display()
        ))
        .arg(cache.join("flutter_tools.snapshot"));
    Ok(cmd)
}

fn status(mut cmd: Command) -> Result<(), String> {
    let shown = format!("{cmd:?}");
    eprintln!("> {shown}");
    let status = cmd.status().map_err(|e| format!("{shown}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{shown} failed ({status})"))
    }
}

/// `args` are what follows the task name: `--release`, then optionally
/// `--` and arguments passed through to Flutter (e.g.
/// `--dart-define=R3D_DEMO=t_shirt`).
pub fn run(root: &Path, task: Task, args: &[String]) -> Result<(), String> {
    let (own, passed) = match args.iter().position(|a| a == "--") {
        Some(i) => (
            args.get(..i).unwrap_or_default(),
            args.get(i + 1..).unwrap_or_default(),
        ),
        None => (args, &[][..]),
    };
    let mut release = false;
    for arg in own {
        match arg.as_str() {
            "--release" => release = true,
            other => return Err(format!("unknown option {other} (expected --release or --)")),
        }
    }
    let os = desktop()?;
    let mode = if release { "--release" } else { "--debug" };
    let app = root.join("app");

    let mut cli = Command::new(env!("CARGO"));
    cli.current_dir(root).args(["build", "-p", "r3d-cli"]);
    if release {
        cli.arg("--release");
    }

    // The handler does nothing: Flutter gets Ctrl+C too and quits.
    ctrlc::set_handler(|| {}).map_err(|e| format!("Ctrl+C handler: {e}"))?;
    let mut cmd = flutter()?;
    cmd.current_dir(&app);
    match task {
        Task::Run => {
            cmd.args(["run", "-d", os, mode]);
        }
        Task::Build => {
            status(cli)?;
            cmd.args(["build", os, mode]);
        }
    }
    cmd.args(passed);
    status(cmd)
}

/// `make` with no target: the `Makefile`'s leading comment, then each
/// target with its `## ...` comment, read from the `Makefile` itself so the
/// help cannot drift from it.
pub fn make_help(root: &Path) -> Result<(), String> {
    let path = root.join("Makefile");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    for line in text.lines().map_while(|l| l.strip_prefix('#')) {
        eprintln!("{}", line.strip_prefix(' ').unwrap_or(line));
    }
    let targets: Vec<(&str, &str)> = text
        .lines()
        .filter_map(|line| {
            let (target, rest) = line.split_once(':')?;
            let (_, help) = rest.split_once("## ")?;
            let is_name = !target.is_empty()
                && target
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            is_name.then_some((target, help.trim()))
        })
        .collect();
    let width = targets.iter().map(|(t, _)| t.len()).max().unwrap_or(0);
    eprintln!("\nTargets:");
    for (target, help) in targets {
        eprintln!("  make {target:<width$}  {help}");
    }
    Ok(())
}
