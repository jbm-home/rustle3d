# Rustle3D architecture

Status: living document, last updated 2026-11-14.

Every choice here is the **current best guess**, not a rule. Each one says
why it was made and what was considered, so it can be challenged and changed
as we learn. When a choice changes, update this document and add a line to
the change log at the end. Sections marked **Open** are explicitly undecided.

## 1. Goals that shape the architecture

1. **One command layer.** The desktop UI, the CLI and the MCP server are three
   front-ends over the same typed, undoable commands. An AI agent can do
   everything a user can do, and nothing is reachable only from the UI.
2. **A UI that never blocks on physics.** Simulation runs on its own thread.
   The UI shows the latest finished frame.
3. **Testable without a window or GPU** for everything except rendering:
   document, geometry, commands, simulation and file formats.
4. **Open, versioned, readable data.** A project is a folder with a JSON
   document that humans, git and agents can read.

## 2. Crates

```
                 ┌──────────┐   ┌──────────┐
                 │ r3d-app  │   │ r3d-cli  │      front-ends (binaries)
                 └──┬──┬──┬─┘   └─┬──┬──┬──┘
                    │  │  └───────┼──┼──┼───────┐
                    │  │   ┌──────┘  │  │       │
                    │  ▼   ▼         │  ▼       │
                    │ ┌──────────┐   │ ┌────────────┐
                    │ │ r3d-mcp  │───┼▶│ r3d-render │   wgpu, offscreen PNG
                    │ └────┬─────┘   │ └─────┬──────┘
                    ▼      ▼         ▼       │
                 ┌──────────────────────┐    │
                 │     r3d-commands     │    │    commands, undo, Session
                 └──┬───────┬───────┬───┘    │
                    ▼       ▼       ▼        ▼
              ┌────────┐ ┌───────┐ ┌──────────┐
              │ r3d-io │ │r3d-sim│ │ r3d-core │◀─┘
              └──┬──┬──┘ └───┬───┘ └────┬─────┘
                 │  └────────┼──────────┤
                 ▼           ▼          ▼
              ┌──────────────────────────────┐
              │           r3d-geom           │       pure math
              └──────────────────────────────┘
```

| Crate | Depends on | Must not depend on |
|---|---|---|
| `r3d-geom` | `glam`, `spade` (CDT), `kurbo`? (see 7.4) | anything r3d, GUI, GPU |
| `r3d-core` | `r3d-geom`, `serde`, `serde_json`, `schemars` | sim, GUI, GPU |
| `r3d-sim` | `r3d-geom`, `glam`, `rayon` | core, GUI, GPU (for the CPU backend) |
| `r3d-io` | `r3d-core`, `r3d-geom`, `gltf`, `tobj`, PDF writer | sim, GUI, GPU |
| `r3d-commands` | core, geom, sim, io | GUI, GPU |
| `r3d-render` | core, geom, `wgpu`, `image` | commands, GUI toolkit |
| `r3d-mcp` | commands, render, `rmcp`, `tokio` | GUI toolkit |
| `r3d-cli` | commands, render, mcp, `clap` | GUI toolkit |
| `r3d-app` | commands, render, mcp, GUI toolkit (open, see 7.1) | |

Changes to the split suggested in the brief:

- **Added `r3d-render`.** The brief requires PNG renders of the 2D board and
  the 3D view for agents, from the CLI (headless) and the app. If rendering
  lived in `r3d-app`, the CLI would have to depend on the whole GUI. A shared
  renderer crate draws to the app viewports and to an offscreen
  texture in the CLI and MCP, with the same code and the same visuals.
- **`r3d-sim` does not depend on `r3d-core`.** The solver works with a plain
  `SimScene` (particles, triangles, constraints, colliders, materials as
  numbers). `r3d-commands` builds that scene from the document (the
  "sim bridge"). This keeps the solver reusable and benchmarkable on its own,
  and stops document concepts from leaking into the hot loop.
- **The session lives in `r3d-commands`**, not in a separate engine crate. It
  owns the document, the undo history and the simulation thread handle. A
  separate crate can be split out later if it grows too big.
- An `xtask` crate will be added in M1 to generate `docs/commands.md` and the
  JSON schemas, and to run the golden-file updater.

## 3. Document model (`r3d-core`)

```
Project
 ├─ meta: schema_version, name, units (display only), created/modified
 ├─ panels: IdMap<PanelId, Panel>
 │    Panel { name, contour: Vec<EdgeId>, internal_lines, notches,
 │            symmetry: Option<SymmetryLink>, material: MaterialId,
 │            grain_angle, particle_spacing, placement: Placement3d }
 ├─ points:  IdMap<PointId, Point2>            (shared by edges, mm)
 ├─ edges:   IdMap<EdgeId, Edge>               (line or cubic Bézier between two PointIds)
 ├─ seams:   IdMap<SeamId, Seam>               (two edge chains + direction flags)
 ├─ materials: IdMap<MaterialId, FabricMaterial>
 ├─ avatar:  Option<AvatarRef>                 (path inside the project folder + landmarks)
 ├─ arrangement_points: from the avatar, editable
 └─ drape:   Option<DrapeSnapshot>             (last saved simulated positions)
```

- **IDs** are typed newtypes (`PanelId(u64)`, …) allocated from a monotonic
  counter stored in the document. This makes them deterministic, which golden
  tests need, and they are never reused after deletion.
- **Entity maps** (`IdMap` = ordered `BTreeMap`) give stable iteration order
  and therefore stable JSON.
- **Serialization**: `serde` JSON with a top-level `schema_version: u32`.
  Loading runs `migrate_vN_to_vN+1` functions on the raw `serde_json::Value`
  until it reaches the current version, then deserializes into the typed
  model. Golden files for each version live in `crates/r3d-core/tests/golden/`.
- **Units**: 2D in millimetres, `f64`. Display units (mm/cm/in) are only a UI
  preference.
- **Future-ready**: the grading/sizes, layers, trims and textures in the
  roadmap fit as new entity maps plus per-panel references. The model avoids
  "single garment" assumptions: panels belong to a `GarmentId` from day one,
  even though v0.1 only has one garment.

### Project folder

```
MySkirt.r3d/
 ├─ project.json        the document (pretty-printed, stable key order)
 ├─ avatars/            imported avatar meshes (copied in, referenced relatively)
 ├─ exports/            default output location for OBJ/glTF/SVG/PDF/PNG
 └─ .r3d-cache/         derived data (SDF grids, meshes); safe to delete, git-ignored
```

The project is self-contained, so it can be moved, zipped or committed to git
as a whole. All file-system writes from the MCP server are restricted to this
folder.

## 4. Command layer (`r3d-commands`)

```rust
pub enum Command {            // serde-tagged: {"type": "panel.create_rect", ...}
    PanelCreateRect(PanelCreateRect),
    PointMove(PointMove),
    SeamAdd(SeamAdd),
    MaterialAssign(MaterialAssign),
    SimStart(SimStart),
    ExportGltf(ExportGltf),
    // ...
}

pub trait CommandSpec: Serialize + DeserializeOwned + JsonSchema {
    const NAME: &'static str;          // "panel.create_rect"
    const SUMMARY: &'static str;       // one line, used in docs and MCP tool description
    const DESTRUCTIVE: bool = false;   // needs `confirm: true` from MCP/CLI
    fn validate(&self, doc: &Project) -> Result<(), CommandError>;
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError>;
}
```

- **Undo by recorded changes, not by hand-written inverses.** Commands mutate
  the document only through a `Transaction`, which records each entity change
  as `(id, before: Option<T>, after: Option<T>)`. Undo and redo replay these
  diffs, so a command author cannot write a wrong inverse, and batches and
  transactions come for free. The history is a list of `ChangeSet`s with a
  label and an optional merge key, so that dragging a point records one
  undo step and not 300.
- **Batch**: `Command::Batch(Vec<Command>)` runs atomically. If any command
  fails, the whole batch rolls back and becomes one undo step.
- **Queries** (`ProjectSummary`, `ListPanels`, `GetPanelGeometry`,
  `Measure`, `SimState`, …) are a separate `Query` enum. They are read-only
  and have their own schemas, and they are also exposed through the CLI and
  MCP.
- **Schemas**: `schemars` derives the JSON schema of every command and query.
  The `xtask` generates `docs/commands.md`, and `r3d-mcp` builds its tool list
  from the same registry at startup, so there is a single source of truth.
- **Simulation commands** (`sim.start`, `sim.pause`, `sim.reset`,
  `sim.pin`, `sim.step` …) control the simulation thread. They are not
  undoable document edits. `drape.save` copies the current simulated state
  into the document, and that is undoable.
- **Session** = `Project` + `History` + `SimHandle` + project folder path.
  It exposes `execute(Command)`, `query(Query)`, `undo()`, `redo()`, and an
  event stream (`DocumentChanged`, `SelectionChanged`, `SimFrame`) that the
  front-ends subscribe to.
- **Selection** is session state, not document state. It is not saved and not
  undoable. It is shared by the 2D and 3D views, which keeps them in sync, and
  agents can read and set it.

## 5. Data flow

```
 UI event ─┐
 CLI script ├─▶ Command ─▶ validate ─▶ Transaction ─▶ Project (core)
 MCP tool ─┘                             │
                                         ├─▶ History (undo/redo)
                                         └─▶ Events ─▶ UI repaint / MCP notification
                                                 │
                     geometry or material change ▼
                         sim bridge: core → geom meshing → SimScene
                                                 │
                                                 ▼
                               Simulation thread (r3d-sim)
                                                 │ positions (triple buffer)
                                                 ▼
                                   r3d-render (on-screen / PNG)
```

## 6. Threading model

| Thread | Owns | Talks via |
|---|---|---|
| **UI / main** | `Session` (document, history, selection) | Executes commands synchronously, which is cheap. Reads sim frames without blocking. |
| **Simulation** | `SimScene`, solver state | `crossbeam` channel in (`SimControl`: rebuild, start/pause, pin, grab target, params). Triple buffer out (latest positions plus stats). |
| **Rayon pool** | n/a | Used by the solver inside a step (parallel per constraint color). |
| **Remote/MCP** (app only, tokio) | MCP server on a local socket | Sends `(Command, reply_tx)` to the UI thread over a channel. The UI drains it each frame and calls `request_repaint`. |
| **Background jobs** | SDF building, exports, triangulation of large panels | Job queue. Results come back as commands or events. |

- In headless mode (CLI/MCP over stdio), the same `Session` runs on the tokio
  main task, and `sim.step N` can run the simulation synchronously in
  deterministic mode.
- The document is never shared between threads. It lives on one thread, and
  other threads send messages. This removes lock contention and data races
  by construction.
- Rebuilding the simulation after a geometry edit is incremental where
  possible (only the edited panel is remeshed). Positions are carried over by
  barycentric remapping so that the drape does not jump.

## 7. Key technical choices

### 7.1 GUI stack — **Open** (to be decided by a spike at the start of M3)

A beautiful, modern UI with a very short learning curve is a top-level
product goal (see [`ux.md`](ux.md)). **egui was ruled out** (2026-11-14)
because its look is tool-like and its polish ceiling is too low for that goal.

Candidates to evaluate:

| Candidate | Strengths | Concerns |
|---|---|---|
| **Tauri 2 + web UI** (TypeScript, Svelte or React, CSS) | Highest polish ceiling, huge ecosystem (animation, a11y, i18n, design systems), easy to theme | Two languages; the 3D view has to be either rendered in the webview (WebGPU/WebGL fed with positions over IPC) or composited from native wgpu |
| **Slint** (+ wgpu viewports) | Declarative, designer-friendly, good-looking, Rust-native, built-in translations | Smaller ecosystem; custom wgpu viewport integration must be proven |
| **Iced** (+ `shader` widget) | Pure Rust, wgpu-based, Elm architecture, custom widgets render straight into wgpu | Styling and animations take more work to look premium |
| Others to watch | Dioxus native (Blitz), Freya, Xilem | Young or immature for a large app today |

**Spike protocol**: build the same small screen in the 2–3 strongest
candidates (board with a few panels, 3D view with a 10k-vertex mesh updated
every frame, inspector, light and dark themes). Compare visual quality (the
maintainer's review), 3D frame time and latency, effort per screen, i18n and
accessibility support, and cross-platform behavior. Record the result in the
change log.

Whatever wins, the front-end stays a thin client of `r3d-commands`: it renders
state and emits JSON-serializable commands. That is also why a web front-end
is realistic. The same command protocol already serves the CLI and the MCP
server.

### 7.2 Rendering: our own wgpu pipelines in `r3d-render`

Two renderers: the board renderer (instanced lines and curves, points, fills,
text with `glyphon`) and the 3D renderer
(avatar mesh, cloth mesh with per-vertex normals recomputed each frame, simple
PBR-lite shading, seam lines, pins). Offscreen rendering uses the same
pipelines and writes PNG with `image`. Headless GPU: wgpu works without a
window on macOS and Windows, and on Linux CI uses lavapipe/llvmpipe (software
Vulkan) so that render golden tests can still run.

The offscreen renderer is needed whatever the GUI stack (agents need PNGs from
the headless CLI). How the desktop app shows the views on screen depends on
the outcome of 7.1. With a web front-end, the on-screen views could be drawn
in the webview instead, and kept visually consistent with the offscreen ones.

### 7.3 Simulation: XPBD on CPU with rayon — current choice

See [`physics.md`](physics.md). It sits behind a `Solver` trait, so a GPU
backend (wgpu compute) or Vertex Block Descent can come later without touching
the callers.

### 7.4 Geometry

- `glam` for vectors, `f64` (`DVec2`) in 2D and `f32` (`Vec3`) in the simulation.
- Our own small Bézier module (cubic segments, arc-length tables, splitting,
  nearest point). `kurbo` (MIT/Apache) is a candidate for offsets and
  booleans. We will decide in M2 after a prototype. **Open**
- Constrained Delaunay triangulation with `spade` (MIT/Apache), with interior
  Steiner points on a jittered or hexagonal grid at the panel's
  `particle_spacing`, and boundary resampling that keeps seam vertices on
  both edges in matching count.

### 7.5 MCP: the official Rust SDK (`rmcp`)

- **One MCP implementation, two transports.** In headless mode, `r3d mcp`
  serves stdio over its own `Session`. In attached mode, the desktop app hosts
  the same server on a local socket (a Unix domain socket under the user's
  runtime directory, or a Windows named pipe), and `r3d mcp --attach` is a thin
  stdio↔socket relay. The agent configuration is therefore always
  "run `r3d mcp`", and the only difference is the `--attach` flag.
- Tools are generated from the command and query registry: one tool per
  command (`panel_create_rect`, …), plus `batch`, `undo`, `redo` and query
  tools, plus `render_board` and `render_3d` (which return PNG image
  content).
- **Safety**: paths are resolved and canonicalized against the project folder,
  and anything outside it is refused. Destructive commands (`DESTRUCTIVE =
  true`: delete panel, overwrite file, new project discarding changes) need
  `"confirm": true`. In attached mode, a UI badge shows when an agent is
  connected.

### 7.6 Errors and never-crash

Each crate has its own `thiserror` enum. The app catches errors at the command
boundary and shows them as non-blocking notifications. `catch_unwind` around
the simulation step is a last resort: it resets the simulation and keeps the
document. Clippy denies `unwrap`/`expect`/`panic` outside tests.

### 7.7 Internationalisation — current choice

English is the default UI language. Other languages come later, but the code
must be **i18n-ready from day one**:

- **No user-facing prose is built in the core crates.** Errors and messages
  from `r3d-core` / `r3d-commands` are structured: a stable `code`
  (`"entity_not_found"`) plus named `args` (`{"id": "panel:3"}`). The English
  `Display` text is only a fallback for logs and the CLI. Front-ends translate
  from `code` + `args`.
- **Front-end strings** go through a message catalog keyed by stable ids
  (Fluent `.ftl` files, `en` first). The format follows the GUI stack (7.1),
  but the keys and the `code` + `args` contract do not change.
- **Not translated**: command names, query names, JSON schemas and file
  formats. They are part of the agent/scripting API and stay in English.
- **Locale-aware formatting** for numbers and units in the UI (decimal
  separator). Stored data stays locale-independent (mm, `.` decimals).
- User data such as panel names is never translated. Default names created by
  commands ("Panel 1") come from an `en` default and can later take a locale
  parameter.

## 8. Testing strategy (summary)

| Layer | Tests |
|---|---|
| geom | unit + `proptest` (triangulation covers the polygon, no inverted triangles, spacing within bounds) |
| core | golden JSON per schema version, migration tests, round-trip |
| commands | apply/undo/redo round-trip for every command, batch rollback, schema snapshot test |
| sim | deterministic reference scenes: hanging cloth, cloth on sphere, skirt on avatar; checks for NaN, energy, penetration, seam gaps; `criterion` benchmarks |
| io | export → re-import round-trip; SVG/PDF scale check (a 100 mm test square) |
| render | offscreen PNG golden images with a perceptual tolerance (Linux via lavapipe) |
| mcp/cli | end-to-end scripts that build a skirt and assert on query results |

## 9. Change log of decisions

| Date | Change | Why |
|---|---|---|
| 2026-11-14 | Initial choices: XPBD on CPU, folder projects, MakeHuman CC0 avatars, `r3d-render` crate added | Bootstrap discussion |
| 2026-11-14 | egui ruled out; GUI stack is open until the M3 spike | Not modern or polished enough for the "beautiful UI" goal |
| 2026-11-14 | i18n-ready from M1: structured error codes + args, catalog-keyed UI strings | English default, other languages later without refactoring |
