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
    /// The user's library folder, where `library.*` steps find saved
    /// patterns (default: the desktop app's, shared with it).
    #[arg(long, global = true)]
    library: Option<PathBuf>,
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
    /// The draped garment ready for a game engine (GLB with LODs, baked
    /// normal maps and skinning to the avatar; see --preset).
    Game,
}

#[derive(Clone, Copy, ValueEnum)]
enum PresetArg {
    Generic,
    Godot,
    Unity,
    Unreal,
}

/// Options of `export --format game`.
#[derive(clap::Args)]
struct GameArgs {
    /// Game export: the target engine (sets the LODs, map size, names).
    #[arg(long, value_enum, default_value = "generic")]
    preset: PresetArg,
    /// Game export: LOD ratios, comma-separated, the first 1 (e.g.
    /// 1,0.5,0.2).
    #[arg(long)]
    lods: Option<String>,
    /// Game export: normal map size in pixels (0: none; 256-4096).
    #[arg(long)]
    normal_map: Option<u32>,
    /// Game export: a thin shell instead of double-sided faces.
    #[arg(long)]
    shell: bool,
    /// Game export: the shell's thickness (mm).
    #[arg(long)]
    thickness_mm: Option<f64>,
    /// Game export: do not skin the garment to the avatar's skeleton.
    #[arg(long)]
    no_skin: bool,
    /// Game export: skin without the motions as animations.
    #[arg(long)]
    no_animations: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum MarkerFormatArg {
    Svg,
    Pdf,
}

#[derive(Clone, Copy, ValueEnum)]
enum TechPackViewsArg {
    /// The 3D view, front and back (fast).
    #[value(name = "3d")]
    View3d,
    /// Quick photos, front and back.
    Photo,
    /// No pictures of the garment.
    None,
}

#[derive(Clone, Copy, ValueEnum)]
enum TechPackPaperArg {
    A4,
    Letter,
}

#[derive(Subcommand)]
enum AvatarCmd {
    /// Import a body from an OBJ, glTF (buffers embedded), GLB, PLY or STL file,
    /// then save: the file is copied into the project's `avatars/`
    /// folder; units, up axis and facing are detected unless given;
    /// prints the measurements taken on it and its check.
    Import {
        project: PathBuf,
        /// The .obj, .gltf, .glb, .ply or .stl file.
        file: PathBuf,
        /// Metres per file unit (0.01 for centimetres).
        #[arg(long)]
        scale: Option<f64>,
        /// The file's up axis: y, z, x, -y, -z or -x.
        #[arg(long, allow_hyphen_values = true)]
        up: Option<String>,
        /// Turn about the vertical (degrees) so the body faces +Z.
        #[arg(long, allow_negative_numbers = true)]
        turn: Option<f64>,
    },
    /// Check the imported body before draping: holes, scale, orientation,
    /// arrangement points.
    Check { project: PathBuf },
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
enum FitMapArg {
    /// Stretch beyond the pattern, 0 (dark) to 20 % (yellow).
    Strain,
    /// Distance to the body, 50 mm (dark) to 0 (yellow).
    Gap,
}

impl From<FitMapArg> for r3d_commands::fit::FitMap {
    fn from(m: FitMapArg) -> Self {
        match m {
            FitMapArg::Strain => Self::Strain,
            FitMapArg::Gap => Self::Gap,
        }
    }
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
    /// DXF, SVG, PDF, or a Seamly2D .sm2d or Valentina .val pattern), then
    /// save. The file is copied into the project's `imports/` folder
    /// first, with the measurement file a Seamly2D/Valentina pattern
    /// names. One undo step.
    Import {
        project: PathBuf,
        /// The .dxf, .svg, .pdf, .sm2d or .val file.
        file: PathBuf,
        /// Seamly2D/Valentina: the measurement file to draft with, in
        /// place of the one the pattern names.
        #[arg(long)]
        measurements: Option<PathBuf>,
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
    /// Take the avatar's body from a Seamly2D or Valentina measurement
    /// file (.vit, .smis; a multisize .vst or .smms also sets the size
    /// range), then save. The file is copied into `imports/` first.
    Measurements {
        project: PathBuf,
        /// The measurement file.
        file: PathBuf,
    },
    /// Drape on a body imported from a file (a scan, a game character;
    /// you own its licence), or check it.
    Avatar {
        #[command(subcommand)]
        cmd: AvatarCmd,
    },
    /// Put a print on a garment's fabric, then save: a PNG or JPEG file
    /// (copied into the project's `textures/` folder) or a built-in print
    /// (`stripes`, `gingham`, `dots`, `twill`, in the garment's colour).
    /// It lies along each piece's grainline at its real size.
    Texture {
        project: PathBuf,
        /// The image file, or the name of a built-in print.
        print: String,
        /// The garment (`garment:N`); default: the project's first.
        #[arg(long)]
        garment: Option<String>,
        /// Width of one repeat across the grain (mm); default: the
        /// built-in's usual one, 100 mm for an image.
        #[arg(long)]
        width_mm: Option<f64>,
        /// Height of one repeat along the grain (mm); default: the
        /// image's proportions.
        #[arg(long)]
        height_mm: Option<f64>,
        /// The print turned from the grainline (degrees).
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        angle: f64,
    },
    /// Export the pattern at 1:1 scale (svg, pdf, dxf) or the draped garment
    /// (glb, obj, game; the cached `simulate` result if up to date) into
    /// the project folder.
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
        #[command(flatten)]
        game: GameArgs,
    },
    /// Lay a garment's pieces out on the fabric (a marker): length,
    /// efficiency, yardage and cost; with --format, also write it at 1:1
    /// into the project folder.
    Marker {
        project: PathBuf,
        /// Sizes of the range, comma-separated, or `all` (default: the
        /// pattern as drawn).
        #[arg(long)]
        sizes: Option<String>,
        /// Usable fabric width in cm (default: the garment's, else 150).
        #[arg(long)]
        width_cm: Option<f64>,
        /// The garment (default: the project's only one).
        #[arg(long)]
        garment: Option<String>,
        /// Write the marker as svg or pdf.
        #[arg(long, value_enum)]
        format: Option<MarkerFormatArg>,
        /// PDF page layout (default: one page).
        #[arg(long, value_enum, default_value = "single")]
        paper: PaperArg,
        /// Output path relative to the project (default:
        /// exports/<name>-marker.<ext>).
        #[arg(long)]
        out: Option<String>,
    },
    /// Write the project's tech pack (PDF) into the project folder: views
    /// of the garment, colourways, bill of materials with the fabric per
    /// garment, measurements, the pattern and the sewing notes.
    Techpack {
        project: PathBuf,
        /// Pictures of the garment, front and back: rendered from the
        /// cached drape (`simulate` first) or where the simulation starts.
        #[arg(long, value_enum, default_value = "3d")]
        views: TechPackViewsArg,
        /// Landscape page size.
        #[arg(long, value_enum, default_value = "a4")]
        paper: TechPackPaperArg,
        /// Output path relative to the project (default:
        /// exports/<name>-techpack.pdf).
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
        /// Move the avatar after 1.5 s of draping: walk, turn,
        /// raise_arms, idle, or a BVH motion capture file (copied into
        /// the project's motions/ folder).
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
        /// 3D: colour the cloth by how tight it is (a bar at the right
        /// edge is the scale: yellow tight, dark loose).
        #[arg(long, value_enum)]
        fit_map: Option<FitMapArg>,
        #[command(flatten)]
        photo: PhotoArgs,
    },
    /// How the draped garment fits (the cached `simulate` result): strain
    /// per piece, the tight spots, and the ease at the chest, waist and
    /// hips.
    Fit {
        project: PathBuf,
        /// Strain (%) from which the cloth counts as tight.
        #[arg(long, default_value_t = 5.0)]
        threshold_pct: f64,
        /// More heights above the floor (cm) to take the tape at,
        /// comma-separated.
        #[arg(long, value_delimiter = ',')]
        heights_cm: Vec<f64>,
    },
    /// Where the drape goes wrong and why (the cached `simulate` result):
    /// per face its strain, area, distance to the body, nearby cloth and
    /// seam gaps; every seam's gap; the worst faces.
    Diagnose {
        project: PathBuf,
        /// Faces listed per list (1-200).
        #[arg(long, default_value_t = 20)]
        limit: u32,
        /// Regions to report on, as the JSON array of `drape.diagnose`
        /// (e.g. '[{"kind":"drape","point_m":[0,1.2,0.1],"radius_mm":30}]').
        #[arg(long)]
        regions: Option<String>,
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

fn cmd_run(
    project: &Path,
    script: &Path,
    dry_run: bool,
    library: Option<PathBuf>,
) -> Result<(), Failure> {
    let script = Script::parse(&read_script(script)?)?;
    let mut ws = Workspace::open(project)?;
    ws.set_library(library);
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
    game: GameArgs,
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
        Format::Game => {
            let mut step = json!({
                "type": "export.game",
                "avatar": avatar,
                "preset": match game.preset {
                    PresetArg::Generic => "generic",
                    PresetArg::Godot => "godot",
                    PresetArg::Unity => "unity",
                    PresetArg::Unreal => "unreal",
                },
                "sides": if game.shell { "shell" } else { "double_sided" },
                "skin": !game.no_skin,
                "animations": !game.no_animations,
            });
            if let Some(lods) = &game.lods {
                let ratios: Result<Vec<f64>, _> =
                    lods.split(',').map(|r| r.trim().parse::<f64>()).collect();
                step["lods"] = json!(ratios.map_err(|e| CommandError::Malformed {
                    name: "lods".into(),
                    detail: e.to_string(),
                })?);
            }
            if let Some(size) = game.normal_map {
                step["normal_map"] = json!(size);
            }
            if let Some(mm) = game.thickness_mm {
                step["thickness_mm"] = json!(mm);
            }
            step
        }
    };
    if let Some(path) = out {
        step["path"] = Value::from(path);
    }
    let mut ws = Workspace::open(project)?;
    if let Some(sizes) = sizes {
        step["sizes"] = json!(size_names(&ws, &sizes));
    }
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

/// The size names given as `--sizes`: comma-separated, or `all`.
fn size_names(ws: &Workspace, sizes: &str) -> Vec<String> {
    if sizes.trim() == "all" {
        ws.session()
            .project()
            .grading()
            .map(|g| g.sizes.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
    } else {
        sizes.split(',').map(|s| s.trim().to_owned()).collect()
    }
}

struct MarkerArgs {
    sizes: Option<String>,
    width_cm: Option<f64>,
    garment: Option<String>,
    format: Option<MarkerFormatArg>,
    paper: PaperArg,
    out: Option<String>,
}

fn cmd_marker(project: &Path, a: MarkerArgs) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let mut spec = json!({});
    if let Some(sizes) = &a.sizes {
        spec["sizes"] = json!(size_names(&ws, sizes));
    }
    if let Some(cm) = a.width_cm {
        spec["width_mm"] = json!(cm * 10.0);
    }
    if let Some(g) = a.garment {
        spec["garment"] = json!(g);
    }
    let step = match a.format {
        None => json!({ "type": "marker.compute", "marker": spec }),
        Some(f) => {
            let mut step = json!({
                "type": "export.marker",
                "marker": spec,
                "format": match f {
                    MarkerFormatArg::Svg => "svg",
                    MarkerFormatArg::Pdf => "pdf",
                },
                "paper": match a.paper {
                    PaperArg::A4 => "a4",
                    PaperArg::Letter => "letter",
                    PaperArg::Single => "single",
                },
            });
            if let Some(path) = a.out {
                step["path"] = Value::from(path);
            }
            step
        }
    };
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

fn cmd_techpack(
    project: &Path,
    views: TechPackViewsArg,
    paper: TechPackPaperArg,
    out: Option<String>,
) -> Result<(), Failure> {
    let ws = Workspace::open(project)?;
    let request = r3d_commands::techpack::ExportTechPack {
        paper: match paper {
            TechPackPaperArg::A4 => r3d_commands::techpack::TechPackPaper::A4,
            TechPackPaperArg::Letter => r3d_commands::techpack::TechPackPaper::Letter,
        },
        path: out,
        ..Default::default()
    };
    let views = match views {
        TechPackViewsArg::View3d => render::TechPackViews::View3d,
        TechPackViewsArg::Photo => render::TechPackViews::Photo,
        TechPackViewsArg::None => render::TechPackViews::None,
    };
    let pictures = if views == render::TechPackViews::None {
        Vec::new()
    } else {
        let gpu = render::headless_gpu()?;
        render::techpack_views(&gpu, &ws, views, &request)?
    };
    print_json(&request.write(&ws, pictures)?)
}

fn cmd_import(
    project: &Path,
    file: &Path,
    measurements: Option<&Path>,
    units: Option<UnitArg>,
    scale: Option<f64>,
    page: Option<u32>,
) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let (path, linked) = ws.adopt_pattern(file)?;
    let mut step = json!({ "type": "import.pattern", "path": path });
    let measurements = match measurements {
        Some(m) => Some(ws.adopt(m, "imports")?),
        None => linked,
    };
    if let Some(m) = measurements {
        step["measurements"] = Value::from(m);
    }
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

fn cmd_measurements(project: &Path, file: &Path) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let path = ws.adopt(file, "imports")?;
    let result = ws.run_step(&Step::from_value(
        json!({ "type": "import.measurements", "path": path }),
    )?)?;
    ws.save()?;
    print_json(&result)
}

fn cmd_avatar(cmd: AvatarCmd) -> Result<(), Failure> {
    match cmd {
        AvatarCmd::Import {
            project,
            file,
            scale,
            up,
            turn,
        } => {
            let mut ws = Workspace::open(&project)?;
            let path = ws.adopt_body(&file)?;
            let mut step = json!({ "type": "avatar.import", "path": path });
            if let Some(s) = scale {
                step["scale"] = json!(s);
            }
            if let Some(u) = up {
                step["up"] = json!(u);
            }
            if let Some(t) = turn {
                step["turn_deg"] = json!(t);
            }
            let result = ws.run_step(&Step::from_value(step)?)?;
            ws.save()?;
            print_json(&result)
        }
        AvatarCmd::Check { project } => {
            let mut ws = Workspace::open(&project)?;
            let step = Step::from_value(json!({ "type": "avatar.check" }))?;
            print_json(&ws.run_step(&step)?)
        }
    }
}

fn cmd_texture(
    project: &Path,
    print: &str,
    garment: Option<String>,
    width_mm: Option<f64>,
    height_mm: Option<f64>,
    angle: f64,
) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let builtin = ["stripes", "gingham", "dots", "twill"].contains(&print);
    let source = if builtin && !Path::new(print).exists() {
        json!({ "builtin": print })
    } else {
        json!({ "image": ws.adopt_texture(Path::new(print))? })
    };
    let mut step = json!({ "type": "garment.set_print", "print": source, "angle_deg": angle });
    for (key, value) in [("width_mm", width_mm), ("height_mm", height_mm)] {
        if let Some(v) = value {
            step[key] = json!(v);
        }
    }
    if let Some(g) = garment {
        step["garment"] = json!(g);
    }
    ws.run_step(&Step::from_value(step)?)?;
    ws.save()?;
    let summary = ws.run_step(&Step::from_value(json!({ "type": "project.summary" }))?)?;
    print_json(&json!({ "garments": summary["garments"] }))
}

fn cmd_simulate(project: &Path, frames: u32, motion: Option<String>) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let mut step = json!({ "type": "simulate", "frames": frames });
    match motion {
        Some(m) if m.to_ascii_lowercase().ends_with(".bvh") => {
            step["motion_file"] = json!(ws.adopt_motion(Path::new(&m))?);
        }
        Some(m) => step["motion"] = json!(m),
        None => {}
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

fn cmd_fit(project: &Path, threshold_pct: f64, heights_cm: Vec<f64>) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let step = json!({
        "type": "fit.report",
        "threshold_pct": threshold_pct,
        "heights_cm": heights_cm,
    });
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

fn cmd_diagnose(project: &Path, limit: u32, regions: Option<&str>) -> Result<(), Failure> {
    let mut ws = Workspace::open(project)?;
    let regions: Value = match regions {
        Some(text) => serde_json::from_str(text).map_err(|_| {
            r3d_commands::CommandError::invalid("regions", r3d_commands::error::Invalid::Format)
        })?,
        None => json!([]),
    };
    let step = json!({ "type": "drape.diagnose", "limit": limit, "regions": regions });
    let result = ws.run_step(&Step::from_value(step)?)?;
    print_json(&result)
}

fn cmd_render(
    project: &Path,
    view: ViewArg,
    out: &Path,
    size: (u32, u32),
    fit_map: Option<FitMapArg>,
    photo: &PhotoArgs,
) -> Result<(), Failure> {
    if fit_map.is_some() && !matches!(view, ViewArg::Garment) {
        return Err(r3d_commands::CommandError::Malformed {
            name: "fit-map".into(),
            detail: "only with --view 3d".into(),
        }
        .into());
    }
    let ws = Workspace::open(project)?;
    let view = match view {
        ViewArg::Board => render::View::Board,
        ViewArg::Garment => render::View::Garment(fit_map.map(Into::into)),
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

fn cmd_mcp(project: &Path, library: Option<PathBuf>) -> Result<(), Failure> {
    let mut ws = if project.exists() {
        Workspace::open(project)?
    } else {
        Workspace::create(project, &default_name(project))?
    };
    ws.set_library(library);
    r3d_mcp::serve_stdio(ws).map_err(|e| io_failure("mcp", e.to_string()))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let library = cli
        .library
        .clone()
        .or_else(r3d_commands::library::default_dir);
    let result = match cli.cmd {
        Cmd::New { path, name } => cmd_new(&path, name),
        Cmd::Run {
            project,
            script,
            dry_run,
        } => cmd_run(&project, &script, dry_run, library),
        Cmd::Query { project, query } => cmd_query(&project, &query),
        Cmd::Export {
            project,
            format,
            paper,
            avatar,
            sizes,
            out,
            game,
        } => cmd_export(&project, format, paper, avatar, sizes, out, game),
        Cmd::Import {
            project,
            file,
            measurements,
            units,
            scale,
            page,
        } => cmd_import(&project, &file, measurements.as_deref(), units, scale, page),
        Cmd::Measurements { project, file } => cmd_measurements(&project, &file),
        Cmd::Avatar { cmd } => cmd_avatar(cmd),
        Cmd::Texture {
            project,
            print,
            garment,
            width_mm,
            height_mm,
            angle,
        } => cmd_texture(&project, &print, garment, width_mm, height_mm, angle),
        Cmd::Marker {
            project,
            sizes,
            width_cm,
            garment,
            format,
            paper,
            out,
        } => cmd_marker(
            &project,
            MarkerArgs {
                sizes,
                width_cm,
                garment,
                format,
                paper,
                out,
            },
        ),
        Cmd::Techpack {
            project,
            views,
            paper,
            out,
        } => cmd_techpack(&project, views, paper, out),
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
            fit_map,
            photo,
        } => cmd_render(&project, view, &out, size, fit_map, &photo),
        Cmd::Fit {
            project,
            threshold_pct,
            heights_cm,
        } => cmd_fit(&project, threshold_pct, heights_cm),
        Cmd::Diagnose {
            project,
            limit,
            regions,
        } => cmd_diagnose(&project, limit, regions.as_deref()),
        Cmd::Schema { name } => cmd_schema(name.as_deref()),
        Cmd::Mcp {
            project,
            attach,
            endpoint,
        } => match (project, attach) {
            (_, true) => r3d_mcp::attach::relay(endpoint.as_deref())
                .map_err(|e| io_failure("mcp", e.to_string())),
            (Some(project), false) => cmd_mcp(&project, library),
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
