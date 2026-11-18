//! `r3d`: headless Rustle3D command-line interface.
//!
//! Output is JSON on stdout; errors are JSON on stderr
//! (`{"error": {"code", "args", "text"}}`). Exit codes: 0 success,
//! 1 user error (bad input, file problem), 2 internal error.
//! See `docs/specs/10-cli.md`.

use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use r3d_commands::step::parse_query;
use r3d_commands::{
    Action, Command, CommandError, Info, Progress, Query, Script, Session, Status, Step, Workspace,
};
use r3d_core::{Localizable, Message};
use r3d_mcp::render;
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
    /// The pattern at 1:1 scale.
    Svg,
    /// The pattern at 1:1 scale, tiled or on one page.
    Pdf,
    /// The draped garment, glTF 2.0 binary.
    Glb,
    /// The draped garment, Wavefront OBJ.
    Obj,
    /// The pattern as DXF-AAMA (ASTM D6673), for pattern and cutting
    /// software.
    Dxf,
}

#[derive(Clone, Copy, ValueEnum)]
enum UnitArg {
    Mm,
    Cm,
    M,
    In,
    Pt,
    Px,
}

#[derive(Clone, Copy, ValueEnum)]
enum PaperArg {
    A4,
    Letter,
    Single,
}

#[derive(Clone, Copy, ValueEnum)]
enum ViewArg {
    /// The 2D pattern.
    Board,
    /// The garment on the avatar (the cached drape if it is up to date).
    #[value(name = "3d")]
    Garment,
    /// A path-traced photo of the garment on the avatar.
    Photo,
}

#[derive(Clone, Copy, ValueEnum)]
enum QualityArg {
    Quick,
    Good,
    Best,
}

#[derive(Clone, Copy, ValueEnum)]
enum LightingArg {
    Studio,
    Soft,
    Dramatic,
}

#[derive(Clone, Copy, ValueEnum)]
enum CameraArg {
    Front,
    ThreeQuarter,
    Side,
    Back,
}

#[derive(Clone, Copy, ValueEnum)]
enum FrameArg {
    Garment,
    Body,
}

/// Options of `render --view photo`.
#[derive(clap::Args)]
struct PhotoArgs {
    /// Photo: samples per pixel and bounces (quick 16, good 128, best 512).
    #[arg(long, value_enum, default_value = "good")]
    quality: QualityArg,
    /// Photo: studio setup.
    #[arg(long, value_enum, default_value = "studio")]
    lighting: LightingArg,
    /// Photo: where the camera stands.
    #[arg(long, value_enum, default_value = "three-quarter")]
    camera: CameraArg,
    /// Photo: fill the image with the garment or show the whole body.
    #[arg(long, value_enum, default_value = "garment")]
    frame: FrameArg,
    /// Photo: samples per pixel (overrides --quality).
    #[arg(long)]
    samples: Option<u32>,
    /// Photo: light bounces (overrides --quality).
    #[arg(long)]
    bounces: Option<u32>,
    /// Photo: keep the raw samples, without denoising.
    #[arg(long)]
    no_denoise: bool,
}

impl PhotoArgs {
    fn options(&self) -> Result<render::PhotoOptions, Failure> {
        // The MCP tool's names, so both front-ends parse the same way.
        let quality = match self.quality {
            QualityArg::Quick => "quick",
            QualityArg::Good => "good",
            QualityArg::Best => "best",
        };
        let lighting = match self.lighting {
            LightingArg::Studio => "studio",
            LightingArg::Soft => "soft",
            LightingArg::Dramatic => "dramatic",
        };
        let camera = match self.camera {
            CameraArg::Front => render::CameraAngle::Front,
            CameraArg::ThreeQuarter => render::CameraAngle::ThreeQuarter,
            CameraArg::Side => render::CameraAngle::Side,
            CameraArg::Back => render::CameraAngle::Back,
        };
        let frame = match self.frame {
            FrameArg::Garment => "garment",
            FrameArg::Body => "body",
        };
        let mut args = serde_json::Map::new();
        args.insert("quality".into(), quality.into());
        args.insert("lighting".into(), lighting.into());
        args.insert("camera".into(), camera.name().into());
        args.insert("frame".into(), frame.into());
        if let Some(n) = self.samples {
            args.insert("samples".into(), n.into());
        }
        if let Some(n) = self.bounces {
            args.insert("bounces".into(), n.into());
        }
        args.insert("denoise".into(), (!self.no_denoise).into());
        Ok(render::PhotoOptions::from_args(&args)?)
    }
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
    /// Import the pieces of a pattern drawn elsewhere (DXF-AAMA or plain
    /// DXF, SVG, PDF), then save. The file is copied into the project's
    /// `imports/` folder first. One undo step.
    Import {
        project: PathBuf,
        /// The .dxf, .svg or .pdf file.
        file: PathBuf,
        /// What one unit of the drawing is, when the file does not say or
        /// says it wrong.
        #[arg(long, value_enum)]
        units: Option<UnitArg>,
        /// Size factor for a drawing at another scale (2 = drawn at half
        /// size).
        #[arg(long)]
        scale: Option<f64>,
        /// PDF: import only this page (1 = first).
        #[arg(long)]
        page: Option<u32>,
    },
    /// Export the pattern at 1:1 scale (svg, pdf, dxf) or the draped garment
    /// (glb, obj; the cached `simulate` result if up to date) into the
    /// project folder.
    Export {
        project: PathBuf,
        #[arg(long, value_enum)]
        format: Format,
        /// PDF page layout.
        #[arg(long, value_enum, default_value = "a4")]
        paper: PaperArg,
        /// Include the avatar in a 3D export.
        #[arg(long)]
        avatar: bool,
        /// Pattern exports: these sizes of the range, comma-separated (one
        /// alone, or several nested), or `all`.
        #[arg(long)]
        sizes: Option<String>,
        /// Output path relative to the project (default: exports/<name>.<ext>).
        #[arg(long)]
        out: Option<String>,
    },
    /// Drape the placed, sewn garment on the avatar (deterministic) and
    /// cache the result in the project for `render --view 3d`.
    Simulate {
        project: PathBuf,
        /// Frames of 1/60 s (default 300 = 5 s).
        #[arg(long, default_value_t = 300)]
        frames: u32,
        /// Move the avatar after 1.5 s of draping: walk, turn or
        /// raise_arms.
        #[arg(long)]
        motion: Option<String>,
    },
    /// Render the pattern (`board`), the garment on the avatar (`3d`) or a
    /// photorealistic studio image of it (`photo`) to a PNG file.
    Render {
        project: PathBuf,
        #[arg(long, value_enum)]
        view: ViewArg,
        /// Output PNG file.
        #[arg(long)]
        out: PathBuf,
        /// Image size, e.g. 1200x900.
        #[arg(long, default_value = "1200x900", value_parser = parse_size)]
        size: (u32, u32),
        #[command(flatten)]
        photo: PhotoArgs,
    },
    /// List every operation, or print the JSON schema of one.
    Schema { name: Option<String> },
    /// Serve a project to an AI agent over MCP (stdio). With --project, the
    /// folder headless (created if missing; changes saved as they happen).
    /// With --attach, the project open in the desktop app, live: allow
    /// agents in the app first.
    Mcp {
        #[arg(long, required_unless_present = "attach", conflicts_with = "attach")]
        project: Option<PathBuf>,
        /// Relay to the running app instead of serving a folder.
        #[arg(long)]
        attach: bool,
        /// The app's endpoint file (default: where the app writes it).
        #[arg(long, requires = "attach")]
        endpoint: Option<PathBuf>,
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
    let progress = progress();
    let results = script.run(|step| ws.run_step_with(step, &progress))?;
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
    avatar: bool,
    sizes: Option<String>,
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
        Format::Glb => json!({ "type": "export.gltf", "avatar": avatar }),
        Format::Obj => json!({ "type": "export.obj", "avatar": avatar }),
        Format::Dxf => json!({ "type": "export.dxf" }),
    };
    if let Some(path) = out {
        step["path"] = Value::from(path);
    }
    let mut ws = Workspace::open(project)?;
    if let Some(sizes) = sizes {
        let names: Vec<String> = if sizes.trim() == "all" {
            ws.session()
                .project()
                .grading()
                .map(|g| g.sizes.iter().map(|s| s.name.clone()).collect())
                .unwrap_or_default()
        } else {
            sizes.split(',').map(|s| s.trim().to_owned()).collect()
        };
        step["sizes"] = json!(names);
    }
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

fn cmd_import(
    project: &Path,
    file: &Path,
    units: Option<UnitArg>,
    scale: Option<f64>,
    page: Option<u32>,
) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let path = ws.adopt(file)?;
    let mut step = json!({ "type": "import.pattern", "path": path });
    if let Some(u) = units {
        step["units"] = Value::from(match u {
            UnitArg::Mm => "mm",
            UnitArg::Cm => "cm",
            UnitArg::M => "m",
            UnitArg::In => "in",
            UnitArg::Pt => "pt",
            UnitArg::Px => "px",
        });
    }
    if let Some(s) = scale {
        step["scale"] = Value::from(s);
    }
    if let Some(p) = page {
        step["page"] = Value::from(p);
    }
    let result = ws.run_step(&Step::from_value(step)?)?;
    ws.save()?;
    print_json(&result)
}

fn cmd_simulate(project: &Path, frames: u32, motion: Option<String>) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let mut step = json!({ "type": "simulate", "frames": frames });
    if let Some(m) = motion {
        step["motion"] = json!(m);
    }
    let step = Step::from_value(step)?;
    print_json(&ws.run_step_with(&step, &progress())?)
}

/// Where long steps report: a line on stderr when it is a terminal
/// (`drafting sizes: 3/9 (40)`), nothing otherwise (scripts read stdout
/// only).
fn progress() -> Progress {
    if !std::io::stderr().is_terminal() {
        return Progress::default();
    }
    Progress::new(
        |s: &Status| {
            let task = match s.task {
                "template.sizes" => "drafting sizes",
                "simulate.frames" => "draping, frames",
                other => other,
            };
            let item = if s.item.is_empty() {
                String::new()
            } else {
                format!(" ({})", s.item)
            };
            // One line, rewritten as the step goes on.
            eprint!("\r{task}: {}/{}{item}   ", s.done, s.total);
            if s.done == s.total {
                eprintln!();
            }
        },
        std::sync::Arc::default(),
    )
}

fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once('x')
        .ok_or("expected WIDTHxHEIGHT, e.g. 1200x900")?;
    let parse = |v: &str| {
        v.parse::<u32>()
            .ok()
            .filter(|n| render::SIZE_RANGE.contains(n))
            .ok_or("sizes must be 16 to 4096 pixels")
    };
    Ok((parse(w)?, parse(h)?))
}

fn cmd_render(
    project: &Path,
    view: ViewArg,
    out: &Path,
    size: (u32, u32),
    photo: &PhotoArgs,
) -> Result<(), Failure> {
    let ws = Workspace::open(project)?;
    let view = match view {
        ViewArg::Board => render::View::Board,
        ViewArg::Garment => render::View::Garment,
        ViewArg::Photo => render::View::Photo(photo.options()?),
    };
    let gpu = render::headless_gpu()?;
    let (png, mut info) = render::render(&gpu, &ws, view, size.0, size.1)?;
    std::fs::write(out, &png).map_err(|e| Failure {
        message: Message::new(
            "write_failed",
            format!("cannot write {}: {e}", out.display()),
        )
        .arg("path", out.display()),
        code: 1,
    })?;
    info["out"] = Value::from(out.display().to_string());
    info["size"] = json!([size.0, size.1]);
    print_json(&info)
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
            avatar,
            sizes,
            out,
        } => cmd_export(&project, format, paper, avatar, sizes, out),
        Cmd::Import {
            project,
            file,
            units,
            scale,
            page,
        } => cmd_import(&project, &file, units, scale, page),
        Cmd::Simulate {
            project,
            frames,
            motion,
        } => cmd_simulate(&project, frames, motion),
        Cmd::Render {
            project,
            view,
            out,
            size,
            photo,
        } => cmd_render(&project, view, &out, size, &photo),
        Cmd::Schema { name } => cmd_schema(name.as_deref()),
        Cmd::Mcp {
            project,
            attach,
            endpoint,
        } => match (project, attach) {
            (_, true) => r3d_mcp::attach::relay(endpoint.as_deref())
                .map_err(|e| io_failure("mcp", e.to_string())),
            (Some(project), false) => cmd_mcp(&project),
            (None, false) => Err(io_failure("mcp", "--project or --attach is needed".into())),
        },
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
