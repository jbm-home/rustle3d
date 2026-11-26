# Rustle3D architecture

Status: living document, last updated 2026-11-25 (pattern library).

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
5. **Fluid editing in both directions.** Editing the 2D pattern updates the
   drape live, and editing the garment in 3D updates the pattern. The 2D
   pattern stays the single source of truth (see 7.8).
6. **One renderer, from interactive to photorealistic.** The same Rust wgpu
   renderer draws the app's views, the agents' PNG renders and photorealistic
   images, and exports game-ready assets (see 7.9).

## 2. Crates

```
                 ┌──────────┐   ┌──────────┐
                 │ r3d-ffi  │   │ r3d-cli  │      front-ends (app bridge, CLI)
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
| `r3d-geom` | `glam` (later `spade` for triangulation, M4) | anything r3d, GUI, GPU |
| `r3d-core` | `r3d-geom`, `serde`, `serde_json`, `schemars` | sim, GUI, GPU |
| `r3d-sim` | `r3d-geom`, `glam`, `rayon`, `thiserror` | core, GUI, GPU |
| `r3d-sim-gpu` | `r3d-sim`, `r3d-geom`, `wgpu`, `bytemuck`, `pollster`, `thiserror` | core, commands, render, GUI |
| `r3d-io` | `r3d-core`, `r3d-geom`, `glam`, `pdf-writer`, `lopdf`, `roxmltree`, `svgtypes`, `png`, `zune-jpeg`, `miniz_oxide`, `serde_json`, `thiserror` | sim, GUI, GPU; no file I/O (takes and returns bytes) |
| `r3d-commands` | core, geom, io (sim from M4), avatar, `rayon` | GUI, GPU |
| `r3d-render` | core, geom, `wgpu`, `image` | commands, GUI toolkit |
| `r3d-mcp` | commands (render from M3), `rmcp`, `tokio` | GUI toolkit |
| `r3d-cli` | commands, render, mcp, `clap` | GUI toolkit |
| `r3d-ffi` | commands, render, sim-gpu (the app drapes on the GPU), avatar, io (exports), mcp (attached agents), `flutter_rust_bridge` (the Flutter app in `app/`, see 7.1) | |

Changes to the split suggested in the brief:

- **Added `r3d-render`.** The brief requires PNG renders of the 2D board and
  the 3D view for agents, from the CLI (headless) and the app. If rendering
  lived in the app, the CLI would have to depend on the whole GUI. A shared
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
- An `xtask` crate holds developer tasks: `cargo xtask gen-docs`
  regenerates `docs/commands.md` from the registry; `cargo xtask run|build
  [--release]` launches or builds the desktop app for the current OS. The
  root `Makefile` only gives these tasks short names (`make run`, …), so
  nothing depends on make being installed.

## 3. Document model (`r3d-core`)

```
Project                                        (✓ = implemented in M1)
 ├─ name, display_unit, next_id                ✓
 ├─ garments: IdMap<GarmentId, Garment>        ✓ (fabric, colour, layer; M30: fabric details, weight/width/price, notes; M21: print)
 ├─ panels: IdMap<PanelId, Panel>              ✓ (name, garment, contour, grain angle)
 │    Panel { name, contour: Vec<EdgeId>, internal_lines, notches,
 │            symmetry: Option<SymmetryLink>, material: MaterialId,
 │            grain_angle, particle_spacing, placement: Placement3d }
 ├─ points:  IdMap<PointId, Point2>            (shared by edges, mm)
 ├─ edges:   IdMap<EdgeId, Edge>               (line or cubic Bézier between two PointIds)
 ├─ seams:   IdMap<SeamId, Seam>               (two sides: edges or an internal line, a stretch, a direction; ease)
 ├─ materials: IdMap<MaterialId, FabricMaterial>
 ├─ avatar:  Option<Avatar>                    ✓ (measurements; M31: an imported body's file, orientation, arrangement points)
 ├─ grading: Option<Grading>                   ✓ (M22: sizes with body differences, base, shown)
 ├─ constraints: IdMap<ConstraintId, Constraint> ✓ (M27: geometric constraints and dimensions)
 ├─ guides: IdMap<GuideId, Guide>              ✓ (M27: construction lines)
 ├─ variables: name → formula                  ✓ (M28: named variables for formulas)
 ├─ buttons: IdMap<ButtonId, Button>           ✓ (M20: a button, its buttonhole, buttoned or not)
 ├─ zippers: IdMap<ZipperId, Zipper>           ✓ (M20: two sides, an open length from the top)
 ├─ details: IdMap<DetailId, Detail>           ✓ (M34: elastics on a side, a ratio and a width)
 └─ drape:   Option<DrapeSnapshot>             (last saved simulated positions)
```

- **Grading (M22)**: the document holds the pattern in its base size.
  A point's `grade` is its offset in each size (a grade rule table), a
  curved edge's `grade` its handles' (templates set it; otherwise the
  handles follow their points). `Project::at_size(i)` is the whole
  project redrawn in size `i` (based on `i`, the avatar in that size's
  body); `Project::as_shown()` is the one the drape, the 3D view, photos
  and 3D exports use. Derived, never stored: editing stays on one
  geometry, and a size cannot drift from its rules (spec 18).

- **Constraints (M27)** relate contour points and edges (horizontal,
  parallel, tangent, a distance, a length, an angle…). They are document
  entities, edited by commands; the geometry they move stays ordinary
  points and curves, so everything downstream (drape, exports, grading)
  is unchanged. `r3d_core::constraints` turns them into equations and
  solves them with `r3d_geom::solve` (spec 19). A symmetric piece
  (`Panel::symmetry`) is drawn as a half: the reflected half's points
  and edges are derived, paired with the drawn ones so their IDs (and
  the seams on them) survive edits.
- **Formulas (M28)**: a dimension may carry a formula of the body's
  measurements and named variables (`r3d_core::formula`, units checked
  as computed). The stored value is derived from it after every step.
  With a size range, the pieces the formulas drive are solved again in
  every size (that size's body) and the result written as their points'
  grades: derived data in the existing grading model, so `at_size`, the
  graded nest, the exports and the per-size drape need no change.

- **IDs** are typed newtypes (`PanelId(u64)`, …) allocated from one monotonic
  counter per document, shared by every entity kind. They serialize as
  self-describing strings (`"panel:3"`), so an agent cannot pass a point where
  a panel is expected. Allocation is deterministic, which golden tests need.
  Undo restores the counter together with the entities, so IDs are only
  reused when the entities that held them no longer exist anywhere,
  including in the redo stack.
- **Fields are private.** The model is read through accessors and modified
  only through `r3d_core::Transaction` (see §4).
- **Integrity** (`Project::check_integrity`): closed contours, no dangling
  references, finite coordinates, a coherent ID counter. It is checked when a
  file is loaded (a corrupted file is refused, never half-loaded) and after
  every command (a buggy command is rolled back instead of corrupting the
  document).
- **Entity maps** (`IdMap` = ordered `BTreeMap`) give stable iteration order
  and therefore stable JSON.
- **Serialization**: `{"format": "rustle3d.project", "schema_version": N,
  "project": {...}}`. Loading runs `MIGRATIONS[v-1]` (version `v` to `v+1`)
  on the raw `serde_json::Value` until it reaches the current version, then
  deserializes into the typed model and checks integrity. A compile-time
  assertion requires one migration per older version. Golden files for each
  version live in `crates/r3d-core/tests/golden/v{N}/` and are never edited.
  Saving is atomic (temporary file, then rename).
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
 ├─ avatars/            imported avatar meshes (copied in, referenced relatively; spec 27)
 ├─ imports/            pattern files imported (DXF, SVG, PDF), kept as given
 ├─ textures/           print images the user added (PNG, JPEG; spec 25)
 ├─ exports/            default output location for OBJ/glTF/SVG/PDF/PNG
 └─ .r3d-cache/         derived data (SDF grids, meshes); safe to delete, git-ignored
```

The project is self-contained, so it can be moved, zipped or committed to git
as a whole. All file-system writes from the MCP server are restricted to this
folder.

## 4. Command layer (`r3d-commands`)

```rust
// registry.rs: the single list of operations. One line per command; the
// macro generates the serde-tagged `Command` enum, `Named::NAME` and the
// `Command::registry()` used by docs, CLI and MCP.
command_registry! {
    "panel.create_rect" => PanelCreateRect,
    "point.move" => PointMove,
    // ...
}

pub trait CommandSpec: Named + Described + Serialize + DeserializeOwned + JsonSchema {
    const DESTRUCTIVE: bool = false;   // needs `confirm: true`
    /// Validate, then mutate through the transaction. An error drops the
    /// transaction, which rolls back any partial change.
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError>;
    /// Long commands (seconds) override this one: report to `progress`,
    /// stop with `Cancelled` when it asks. The default calls `apply`.
    fn apply_with(&self, tx: &mut Transaction<'_>, progress: &Progress)
        -> Result<CommandOutput, CommandError> { self.apply(tx) }
    fn merge_key(&self) -> Option<String> { None }   // e.g. "point.move:point:3"
    fn confirmed(&self) -> bool { false }
}
```

Validation lives at the start of `apply` rather than in a separate
`validate` step. One code path means validation cannot drift from what
`apply` assumes, and the transaction makes early returns safe.

- **Undo by recorded changes, not by hand-written inverses.** Commands mutate
  the document only through a `Transaction`, which records each entity change
  as `(id, before: Option<T>, after: Option<T>)`. Undo and redo replay these
  diffs, so a command author cannot write a wrong inverse, and batches and
  transactions come for free. `Transaction` and `ChangeSet` live in
  `r3d-core`, next to the private fields they modify. The history
  (`r3d-commands`) is a list of `ChangeSet`s with a label and an optional
  merge key. Merged change sets are compacted per entity (first `before`,
  last `after`), so dragging a point for 300 frames is one small undo step.
  `Session::seal_history` ends a merge run (e.g. on mouse release).
- **Steps**: scripts and agents send *steps*. A step is a command, a query,
  `history.undo`, `history.redo`, `history.list`, `history.go_to`, or
  `batch` (`{"type": "batch", "label": "...", "commands": [...]}`), which
  runs atomically as one undo step named by its label. If any command in a
  batch fails, nothing is applied and the error carries the index of the
  failing command. Undo, redo and go-to return the panels the steps
  changed (`ChangeSet::panels`); the app selects them.
- **Progress of long steps** (M26): a `Progress` (a report callback and a
  cancel flag) goes down `Workspace::run_step_with` → `Session` →
  `CommandSpec::apply_with` / `ActionSpec::run_with`. A template drafted
  in every size reports each size (`template.sizes`), the headless drape
  every ten frames (`simulate.frames`); both stop between parts with
  `CommandError::Cancelled`, and the dropped transaction changes nothing.
  Each front-end chooses the display: the app a card with a bar and Stop,
  the CLI a line on stderr (a terminal only), the MCP server
  `notifications/progress` when the client sent a progress token, and a
  cancelled MCP request stops the step.
- **Queries** (`ProjectSummary`, `ListPanels`, `GetPanelGeometry`,
  `Measure`, `SimState`, …) are a separate `Query` enum. They are read-only
  and have their own schemas, and they are also exposed through the CLI and
  MCP.
- **Schemas**: `schemars` derives the JSON schema of every command and query.
  The `xtask` generates `docs/commands.md`, and `r3d-mcp` builds its tool list
  from the same registry at startup, so there is a single source of truth.
- **Actions** (`export.svg`, `export.pdf`, …) are a third registry next to
  commands and queries: operations that read or write files. They run on
  a `Workspace`, never on a bare `Session`. They are not undoable
  themselves; `import.pattern` reads a file inside the project folder and
  runs the undoable `pattern.import` command with what it found, so an
  import is one undo step and an agent can also send pieces directly.
- **Workspace** = `Session` + project folder. It owns saving (atomic, with a
  `project.vN.json.bak` backup on the first save after a migration) and every
  file write, confined to the project folder: relative paths only, no `..`,
  and symlinks are resolved so they cannot point outside. The CLI and the MCP
  server work on a `Workspace`.
- **Constraints are settled after every step** (M27), in the step's
  own transaction: constraints whose points or edges are gone are
  dropped, then the solver moves the geometry so that every constraint
  holds again, holding first what the step itself moved (read from the
  transaction's changes). A step whose constraints cannot all hold is
  refused (`constraints_unsolvable`) and rolled back. Formulas are
  computed first (the body or a variable may have changed), and with a
  size range the formula-driven pieces are graded after the solve.
  Symmetric pieces'
  reflected halves are then recomputed, then linked mirror copies.
- **Linked mirror copies** are derived data. After every step, the session
  recomputes each copy from its source (reflection across an axis, then an
  offset). It reuses the copy's entity IDs while the source topology is
  unchanged, and writes only values that differ, so an unrelated edit
  records no change on the copies. A copy cannot be edited directly
  (`linked_mirror` error); `panel.translate` moves it through its offset,
  and `panel.unlink` makes it independent.
- **Simulation commands** (`sim.start`, `sim.pause`, `sim.reset`,
  `sim.pin`, `sim.step` …) control the simulation thread. They are not
  undoable document edits. `drape.save` copies the current simulated state
  into the document, and that is undoable.
- **Session** = `Project` + `History` + dirty flag (M1). It will gain the
  `SimHandle` (M4) and an event stream (`DocumentChanged`,
  `SelectionChanged`, `SimFrame`) for front-ends. It exposes `execute`,
  `execute_batch`, `query`, `undo`, `redo` and `run_step`. Persistence stays
  outside, in `r3d_core::project_dir`.
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
| **Rayon pool** | n/a | Used by the CPU solver inside a step (contacts, per particle). The GPU solver records a frame and waits for its readback on the simulation thread. |
| **Remote/MCP** (app only, tokio) | MCP server on a local socket | Sends `(Command, reply_tx)` to the UI thread over a channel. The UI drains it each frame and calls `request_repaint`. |
| **Background jobs** | SDF building, exports, triangulation of large panels | Job queue. Results come back as commands or events. |
| **Long steps** (app, M26) | A step that takes seconds: a template or a size range drafted in every size, new measurements, opening (the body fitted), starting the drape, imports, exports | Called asynchronously over the bridge (a Rust worker thread). The engine records where the step is; the UI polls it (`Engine::progress`) for a card with a bar, and may `Engine::cancel`. The drape is prepared on a copy of the document, so the UI's calls never wait for a body fit. |
| **Body fit** (app, M26) | Fitting the avatar to new measurements (≈ 2.5 s) | A thread of its own when the 3D view needs a body that is not ready; the view keeps the old body meanwhile. `queries::fitted` shares a fit in progress, so no body is fitted twice at once. |

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

### 7.1 GUI stack — Flutter + native Rust renderer

Requirements: a beautiful, accessible UI with a very short learning curve
(see [`ux.md`](ux.md)), fluid 2D ↔ 3D editing, and photorealistic rendering.
**egui was ruled out** (2026-11-14): its look is tool-like and its polish
ceiling too low.

**Current choice: Flutter for the UI, our Rust wgpu renderer for every
view.** Flutter (Dart) draws the chrome: panels, inspector, menus,
animations, onboarding. It has its own GPU renderer, so the UI looks the
same on macOS, Windows and Linux, and it has mature accessibility, i18n and
animation support. The 2D board and the 3D view are rendered by
`r3d-render` (Rust, native wgpu with Metal/D3D12/Vulkan) into a GPU texture
that Flutter composites without copying. The runner creates the texture on
the compositor's side and hands its handle to `r3d-ffi` (`surface.rs`),
which opens the same memory on wgpu's device:
- macOS: an IOSurface, wrapped as a Metal texture;
- Windows: a D3D11 texture on Flutter's adapter, shared through an NT
  handle that wgpu opens as a D3D12 resource (ANGLE opens it on its side).
  The views' device is created on that adapter, found by the LUID the
  runner reports; each frame leaves the texture in the common state;
- Linux: planned (a Vulkan dma-buf behind an `FlTextureGL`).
Elsewhere, or when a texture cannot be opened (another GPU, WSL, a
software GPU), the view renders into a texture of its own that is read
back. Frames render off the UI thread (an asynchronous `render_view`), so
waiting for the GPU never costs the UI a frame. Pointer and keyboard events on these
views go to Rust. Dart talks to the Rust core through `flutter_rust_bridge`,
using the same JSON-serializable commands, queries and events as the CLI
and MCP.

Why this beats the alternatives considered (survey of 2026-11-14):

| Option | Verdict |
|---|---|
| **Tauri 2 + web UI**, our renderer compiled to WASM in the webview | Highest UI polish, but the webview limits rendering to WebGL2 (WebGPU is not guaranteed in Linux webviews), which rules out path tracing and modern GPU features. Ruled out by the photorealism goal. |
| **Iced** | Rust-only (MIT), built on wgpu: our views would share its device with no texture bridge on any system. Active (0.14). Plain default look (fully themeable, as System76's COSMIC shows), fewer animations and less accessibility than Flutter, an API that changes between releases. **Plan B** if Flutter stops being viable. |
| Slint | Rust-only, good-looking, can show wgpu textures. Ruled out by its licence while Rustle3D was MIT (GPLv3, a royalty-free licence of its own or a commercial one). Since the switch to GPL-3.0-or-later its GPLv3 option would be acceptable; Flutter stays the choice on its own merits. |
| GPUI (+ gpui-component) | Superb look, but no wgpu: embedding our renderer is a large project. Windows/Linux not mature. |
| Makepad | Creative-tool oriented with built-in 3D, but sparse documentation and a small community. |
| egui, Dioxus native (Blitz), Xilem | Tool-like look (egui), or pre-alpha / experimental. |
| Bevy | A game engine: it wants to own the renderer and the app's structure (ECS), its UI is far from a dense editor's needs, and its API breaks every few months. |

Costs we accept: a second language (Dart) for the UI layer only, the Flutter
toolchain (and full Xcode on macOS), and our own small texture bridge if the
existing `flutter_wgpu_texture` crate (young, pinned to an old wgpu) does not
fit.

**Spike protocol (start of M3, passed on macOS)**: the same screen in
Flutter + Rust. It shows the board with a few panels, the 3D view with a
10k-vertex mesh updated every frame, an inspector, and light and dark
themes. Pass criteria:
- zero-copy texture on macOS first (Windows and Linux before v0.1);
- input latency under one frame;
- 60 fps with UI animations running;
- a selection made in one view is highlighted in the other;
- the maintainer's visual review.

Whatever wins, the UI stays a thin client of `r3d-commands`.

### 7.2 Rendering: one wgpu renderer, three quality tiers (`r3d-render`)

| Tier | Use | Technique |
|---|---|---|
| **Interactive** | Board and 3D view while editing, agent PNGs | Raster PBR with a cloth BRDF (sheen), soft studio lighting, shadow, MSAA. Fit maps and seam overlays. |
| **Preview** | "Looks good" screenshots | Interactive tier plus screen-space AO and higher-resolution shadows, rendered offscreen at any size. |
| **Photorealistic** (M14) | Marketing-quality images | Progressive GPU path tracer in wgpu compute over a BVH built on the CPU (hardware ray queries later), layered fabric BSDF (sheen, GGX specular, translucency), procedural studio lighting (soft boxes and a seamless backdrop), à-trous denoising, Khronos PBR Neutral tone mapping. Renders on its own thread and refines in the 3D view. Details and references: [`rendering.md`](rendering.md). |

The board renderer draws instanced lines and curves, points, fills, and text
with `glyphon`. Every tier renders offscreen too, for the CLI and MCP.
Headless GPU: wgpu works without a window on macOS and Windows, and Linux CI
uses lavapipe/llvmpipe (software Vulkan) for render golden tests.

### 7.3 Simulation: XPBD behind a `Solver` trait, on the GPU in the app — current choice

See [`physics.md`](physics.md). Two backends run the same compiled
`Model`: `CpuSolver` (`r3d-sim`, rayon; the deterministic reference, used
headless by `simulate`) and `GpuSolver` (`r3d-sim-gpu`, wgpu compute;
used by the app on its shared device, with a fall-back to the CPU).
Vertex Block Descent can come as a third without touching the callers.

### 7.4 Geometry

- `glam` for vectors, `f64` (`DVec2`) in 2D and `f32` (`Vec3`) in the simulation.
- Our own small Bézier module (evaluation, splitting, arc length,
  parameter at length, tangent) and polygon helpers (area, simplicity,
  outward miter offset for seam allowances). This was enough for M2, so
  `kurbo` was not needed. We will revisit if booleans or robust offsets of
  tight curves become necessary.
- **M4**: constrained Delaunay triangulation with `spade` (MIT/Apache), with interior
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
- Tools are generated from the command, query and action registries: one
  tool per operation, with the dots turned into underscores
  (`panel.create_rect` → `panel_create_rect`, as MCP clients require
  `[a-zA-Z0-9_-]`) and the same JSON schema. `history_undo`,
  `history_redo` and `batch` complete the set. `render_board`,
  `render_3d` and `render_photo` return PNG image content (module
  `r3d_mcp::render`, shared with `r3d render`).
- Tool failures are returned as tool results with `isError` and the
  structured error (`code`, `args`, `text`), never as protocol errors, so
  the agent can read them and correct itself. Results are compact JSON.
- **Headless mode (M2)**: `r3d mcp --project <dir>` creates the folder if
  needed and saves after every successful change. Concurrent calls are
  serialized by a lock on the workspace.
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

English is the default UI language; French is the second one (the user
picks the language in the app, remembered in their library folder). The
code stays **i18n-ready** for more:

- **No user-facing prose is built in the core crates.** Errors and messages
  from `r3d-core` / `r3d-commands` are structured: a stable `code`
  (`"entity_not_found"`) plus named `args` (`{"id": "panel:3"}`). The English
  `Display` text is only a fallback for logs and the CLI. Front-ends translate
  from `code` + `args`.
- **Front-end strings** go through a message catalog keyed by stable ids:
  Flutter's ARB files (`app/lib/l10n/app_en.arb` is the template,
  `app_fr.arb` translates it; `flutter gen-l10n` generates the `L` class).
  Core errors are translated in `app/lib/src/i18n.dart` from `code` +
  `args`; a code without a translation shows the English `text`.
- **Shortcuts** are written in messages with the macOS symbols (⌘ ⌥ ⇧ ⌫)
  and shown as the platform's keys elsewhere ("Ctrl+Shift+Z", "Alt-click",
  key names in the UI language); the bindings follow (⌘ on macOS, Ctrl
  elsewhere, plus Ctrl+Y and Delete).
- **Not translated**: command names, query names, JSON schemas and file
  formats. They are part of the agent/scripting API and stay in English.
- **Locale-aware formatting** for numbers and units in the UI (decimal
  separator). Stored data stays locale-independent (mm, `.` decimals).
- User data such as panel names is never translated. Default names created by
  commands ("Panel 1") come from an `en` default; the app names new pieces
  in the UI language (it passes `name`, and renames a template's pieces
  right after drafting, before the history starts).

### 7.8 2D ↔ 3D editing (pattern = single source of truth)

- **2D → 3D**: every pattern edit is a command. The sim bridge remeshes only
  the changed panels, carries positions over by barycentric remapping, and
  the drape keeps going: no reset, no jump. A piece only moved on the
  board (all its corners by the same displacement) is looked up where it
  was: where a piece lies on the board never moves it in 3D.
- **3D → 2D**: every simulated vertex keeps its **pattern coordinates**
  (panel, triangle, barycentric position in the 2D rest shape). A 3D gesture
  is therefore mapped back to the pattern and emitted as ordinary pattern
  commands, then the cloth re-drapes. Planned 3D tools:
  - *Length*: drag a hem or sleeve end, which moves the pattern edge.
  - *Fit*: pinch to take in or let out, which moves the seam points on both
    panels and keeps them the same length.
  - *Draw on the garment*: lines drawn on the 3D surface become internal
    lines (style lines, pocket placement) on the pattern.
  - *Notches and points*: picking in 3D selects the pattern element under
    the cursor.
- Because 3D edits are ordinary commands, they are undoable, scriptable and
  available to agents. There is no separate 3D-only state to keep in sync.
- Picking: the 3D renderer writes an ID buffer (panel and triangle), and the
  barycentric position comes from the hit point. The same IDs drive
  cross-view highlighting.

### 7.9 Export for other tools and game engines (`r3d-io`)

- **glTF 2.0 / GLB** is the main interchange format, read by Unity, Unreal,
  Godot and Blender. UVs come from the pattern (1 UV unit = 1 m of fabric,
  so prints keep their scale). Materials use PBR with `KHR_materials_sheen`
  (with `KHR_texture_transform`), one primitive per panel, and optionally the
  avatar.
- **Game-ready option** (after v0.1): decimated LODs, wrinkle detail baked
  from the full simulation into normal maps, thickness or back faces, and,
  with animated avatars, skinning weights transferred from the avatar
  skeleton.
- **Patterns**: SVG, PDF and DXF-AAMA out; DXF-AAMA (or plain DXF), SVG
  and PDF in (spec 07). All readers produce one `ImportedPattern`; SVG,
  PDF and AAMA-less DXF share the rules that read pieces out of a plain
  drawing (`r3d_io::import::vector`), and polylines are fitted back to
  editable curves (`r3d_geom::fit`, Schneider 1990).
- **OBJ** for simple tools. **OpenUSD** later (open, used by film and
  visualisation pipelines). FBX is excluded: proprietary SDK.

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
| 2026-11-26 | M42: an environment lights a photo (`photo::Environment`, `PhotoScene::with_environment`); its texels (RGBE) and sampling tables go after the prints in the storage buffer the sheen table already shares; the five built-in environments are embedded in `r3d-commands` (`environments::BUILTIN`), resolved with the project's `.hdr` files by `Workspace::environment`; the choice is a photo setting, not saved in the document | No storage binding was free in the trace pass; the shared buffer already decodes packed texels. Embedding as the body pack does keeps CLI, MCP and app on the same files. Like the studio, the environment is how a photo is taken, not part of the garment, so no schema change |
| 2026-11-25 | M39: a child's age on the body (`Body::age_years`, `Size::age_years`, schema v27); the body pack carries MakeHuman's baby and child targets beside the adult's and `BodyShape::age_years` mixes them by age; children's estimates have coefficients of their own, linear in age; templates read a child's proportions through `Stature` (the adult fractions scaled by the body model's child's); a template may be a set (`Template::parts`), each part drafted as a garment of its own; the drape's cloth spacing shrinks with bodies under 1.5 m | One body model for every age keeps fitting, skinning, motions and export unchanged; mixing by age is MakeHuman's own scheme for its targets. A set as several garments keeps each part graded and drafted again by its own template, and the garments' layers keep the top over the bottoms (one garment had no contact between its own layers) |
| 2026-11-25 | M38: a library pattern is a project folder (`patterns/<Name>.r3d/` in the user's library folder, `library.json` beside it for its kind) holding the garment alone with its body and size range; `pattern.add` adds a project document's garments to a project (a clip of each garment pasted under fresh IDs, then drafted again for the body: templates by the difference between two drafts, as Seamly2D patterns already were); the `library.*` actions reach the folder through `Workspace::set_library` (`r3d --library`, default the app's support folder, `r3d_commands::library::support_dirs` shared with the MCP relay); copy and paste carry every entity a piece owns through `r3d_core::Remap` | Being a project, a saved pattern migrates with the schema, opens, renders its thumbnail and zips with code that exists; a clip only pastes into the same schema. Drafting again by difference keeps what the user changed on the pieces. Actions, not queries, because the library is outside the document; one remapping per entity kind, next to the types, replaces the clip's partial copy |
| 2026-11-24 | Friction counts the elastics' tension pressing a particle on the body (`CpuSolver::press`, the GPU's `press` buffer, binding 30, filled by the elastic dispatches and cleared by `collide`); `StepStats::max_speed` is the speed over the frame, not at the last substep | With one iteration a substep a stiff constraint the body keeps from closing pushes the cloth into it some 300 times less than its tension would: elastic waists held nothing (physics.md, Elastic). More iterations would cost every constraint; the tension is known in closed form. The last substep's speed reads an elastic's tenth-of-a-millimetre back and forth as motion |
| 2026-11-24 | `mesh_panel` with lines: the refinement's points are kept only clear of the lines, the piece triangulated again with the lines constrained; line points snapped to near given points and moved off the outline | Lines inserted after the refinement left slivers next to it (quilted cloth jittered at 42 m/s); refining with the lines constrained is not possible (`spade`'s parity flood fill, see 2026-11-18), and refining without the inside/outside test put points just outside the outline |
| 2026-11-24 | Attached agents served concurrently (each on its own task of the host's runtime, a count of those connected; Disconnect closes them all); the Agents guide saves an MCP Bundle (`r3d_mcp::bundle`, MCPB manifest 0.3) whose server is the installed `r3d mcp --attach` by full path, nothing bundled; portable `.zip` packages next to the macOS `.dmg` and the Windows installer | A desktop assistant starts one server for its chat and another for its coding sessions: served one after the other, the second waited and timed out. Pointing the bundle at the installed `r3d` keeps it in step with the app's updates and the relay protocol, and avoids shipping a second unsigned binary that macOS would quarantine. A portable zip lets users run without installing (Python's zipfile on Windows, `ditto` on macOS for the app's links and signature) |
| 2026-11-24 | M34 quilting: `Detail::Quilting` (schema v25); one layer in the solver (`FabricParams::quilted`) with creases on the stitched lines (`SimScene::creases`); the puffs are a per-particle offset of the drape (`Drape::puff`) applied by `Drape::puffed` wherever cloth is drawn or exported, and `LinesOnCloth` works on the puffed cloth | Two simulated layers with a padding volume would double the particles and need volume constraints (maintainer's choice, 2026-11-24); the puff only changes the look, so it stays out of the solver and every front-end gets it from one function |
| 2026-11-24 | M34 interfacing: `Detail::Interfacing` (schema v24) on the whole piece, a band inside outline edges or inside a closed internal line; the solver's interfacing presets and fusing in `r3d_sim::fabric`; `mesh_panel` joins lines that cross at a shared vertex (inserted between their own points, so the lines' returned vertices still match their samples) | An interfacing's edge, shirring rows and sewn lines meet on real pieces; the mesh failed on any crossing before. The numbers of the composite live with the other fabric physics, the document only names kinds |
| 2026-11-24 | M34 topstitching: `Detail::Topstitch` (schema v23); the cloth under details takes a fabric of its own appended to `SimScene::fabrics` per garment and covers (a triangle's `garment` and `fabric` are no longer the same index); stitches drawn as strips of thread on the cloth through the trims' meshes (`r3d_render::trims::meshes` takes them), and `PhotoScene::new` takes those meshes as plain surfaces | One path for every front-end (3D view, renders, tech pack pictures, photos) and no texture atlas per piece; passing the trims' meshes to the photo brought buttons and zippers into photos for free |
| 2026-11-24 | M34: one entity map for the construction details of spec 31 (`Project::details`, `DetailId` "detail:N", `r3d_core::Detail` tagged by kind; schema v22 with elastics), not one map per kind; elastics in the solvers as tension-only distance chains (`SimScene::elastics`, `Model::elastics`) and buckling membranes (warp and weft resist stretching only within 4 cm of an elastic); `r3d_geom::mesh_panel` takes fine zones (lattice points added before the refinement) | The four details share their life cycle (dropped with their piece or side, listed together, changed during a drape), so one map, one change kind and one settle step serve them all. A softer compression compliance did nothing with one iteration per substep (physics.md, Elastic); the lattice keeps the mesher's Delaunay refinement for the grading back to the piece's spacing |
| 2026-11-24 | M35: no separate sketch mode (maintainer's decision: the board already is one); the context actions are a horizontal bar above the views, built in the app from what is picked (`action_bar.dart`), each action an existing command; constraints name notches (`notch_distance`, placed after solving by `notch_places`, not a solver variable) and construction lines (`on_guide`, `guide_distance`: lines are constants of the solver, carried along when a piece that alone uses them moves); dimensions carry a `name` (formulas resolve it through `Scope::dimensions`) and a `label` offset; schema v21 | Notches as solver variables would need their position as a differentiable function of the edge for little gain over a distance along the edge; lines as variables would split every move between the line and the corners, against the drafting habit of a fixed frame |
| 2026-11-23 | M33: the body has fifteen more measurements (`r3d_core::Measure`); only those the user typed are stored (`Avatar.measures`, schema v20), the others estimated by linear formulas in `r3d-core` fitted to what the body model measures (`r3d_avatar::tape`); the avatar fits typed ones through MakeHuman's `measure-*` targets (`fit_typed`); `formula::Scope` takes the `Avatar` | Formulas, sizes and templates need every value without fitting a body (seconds), and the estimates must match the avatar or a sleeve drafted for one arm would not fit the other; storing only typed values keeps estimates current when the four change |
| 2026-11-19 | Calendar versions: the version is the release date, `YY.M.D` (`26.11.19`), set by `cargo xtask version` (or `make version`) in the workspace `Cargo.toml` and `app/pubspec.yaml`; months and days have no leading zero | The maintainer's choice: a version says when it was released. Cargo and Flutter want semver, which forbids leading zeros; two releases on one day are not planned for |
| 2026-11-19 | `cargo xtask run|build` and a thin root `Makefile` launch and build the app on every OS; `xtask` depends on `ctrlc` (MIT/Apache-2.0) and, on Windows, runs the Flutter tool from its snapshot rather than `flutter.bat` | make is not installed on Windows by default, so the logic stays in Rust. Ctrl+C in a terminal killed the task before Flutter had closed the app, and `flutter.bat` then asked "Terminate batch job (Y/N)?" over the shell's prompt; ignoring Ctrl+C needs a console handler, which is `unsafe` without the crate |
| 2026-11-14 | Initial choices: XPBD on CPU, folder projects, MakeHuman CC0 avatars, `r3d-render` crate added | Bootstrap discussion |
| 2026-11-14 | egui ruled out; GUI stack is open until the M3 spike | Not modern or polished enough for the "beautiful UI" goal |
| 2026-11-14 | M1: validation inside `apply` (no separate `validate`); `Transaction`/`ChangeSet` in `r3d-core`; IDs as prefixed strings; batch as a script step rather than a `Command` variant | Fewer code paths; keeps the document's fields private; clearer agent API |
| 2026-11-14 | M2: own Bézier/offset code instead of `kurbo`; triangulation moved to M4 (only the solver needs it); `Workspace` + action registry for file writes; linked mirrors as derived data with offset; MCP tools use underscores and autosave in headless mode | Fewer dependencies and no code written ahead of its use; one place for disk access and path confinement |
| 2026-11-14 | GUI: Flutter UI + native Rust wgpu renderer shown as a zero-copy texture (Slint as plan B); Tauri ruled out | New requirements: fluid 2D ↔ 3D editing, photorealistic rendering, game-engine export, which need a full native GPU renderer shared by the app, agents and offline renders |
| 2026-11-14 | MSRV = current stable (1.100) | An application gains nothing from an old MSRV, which only made Cargo pick older dependencies |
| 2026-11-14 | i18n-ready from M1: structured error codes + args, catalog-keyed UI strings | English default, other languages later without refactoring |
| 2026-11-15 | Solver parallelism: contacts in parallel (rayon, per particle); constraints stay serial Gauss–Seidel. Per-colour parallel constraints measured and dropped | On a garment-sized mesh (1 700 particles, 32 colours × 30 substeps ≈ 1 000 dispatches per frame) per-colour parallelism ran at 45 ms/frame vs 22 ms serial; contacts were 60 % of a step and parallelise coarsely (14 ms/frame, identical results for any thread count) |
| 2026-11-15 | Seams, placements, fabrics (schema v4) and pins (schema v5) are document entities edited by commands | Undoable, saved, and drivable by agents like every other edit; pins survive a restart of the simulation |
| 2026-11-15 | Simulation thread with a mutex-guarded snapshot of positions instead of a triple buffer; controls publish their effect at once | Simple and fast enough at garment sizes; callers see a consistent status as soon as a call returns |
| 2026-11-15 | Avatar SDF signed with angle-weighted pseudo-normals, built in 0.16 s and not cached on disk | The MakeHuman body is a closed manifold; winding numbers and a disk cache stay for imported avatars |
| 2026-11-15 | Templates are scripts in `templates/`, embedded by `r3d-commands` | One source for the app, tests and demos |
| 2026-11-15 | Rendering for agents lives in `r3d_mcp::render`, shared with `r3d render` | `r3d-commands` stays free of GPU code; one place composes the scene |
| 2026-11-16 | Templates are parametric recipes in Rust (`template.apply`), not JSON scripts | Drafts compute from sizes (curves, a sleeve cap solved to the armhole length); scripts could not |
| 2026-11-16 | Schema v6: `Placement.around` (body, left or right arm) | Sleeves start wrapped around the arm they dress |
| 2026-11-16 | Gravity ramps in during the sewing phase | Pieces fell before closing; skirts slid off the waist |
| 2026-11-16 | The user's library (body presets) is outside the project engine in the FFI | The start screen needs it before a project is open |
| 2026-11-16 | Views without a shared texture render into their own and are read back (Linux, Windows) | The app must run on Linux/WSL2 now; zero-copy dma-buf/D3D12 sharing can replace it later |
| 2026-11-16 | `r3d-app` skeleton removed; `r3d-ffi` is the app's bridge crate | The desktop app is the Flutter one (7.1); no dead code |
| 2026-11-16 | UI catalog in ARB (Flutter gen-l10n, `intl`) instead of Fluent; French added, language chosen in the app | ARB is the Flutter stack's native format (7.7 said the format follows the GUI stack); keys and the `code` + `args` contract are unchanged |
| 2026-11-16 | glTF (GLB) and OBJ written by `r3d-io` itself, not the `gltf`/`tobj` crates; fabric colours and sheen moved to `r3d-core` (`Fabric::appearance`) | Writing is simple (a JSON chunk and a binary chunk, a text file); the crates are mostly readers. The renderer and the exports must agree on a fabric's look, and `r3d-io` cannot depend on the GPU crate |
| 2026-11-16 | Attached MCP over loopback TCP with a token in `agent.json` (support folder), opt-in from the app; `r3d mcp --attach` is a byte relay | Works the same on macOS, Linux and Windows (no Unix sockets or named pipes); the token keeps other local programs out; nothing listens unless the user allows it; the relay needs no MCP knowledge, so the app's server is the only one |
| 2026-11-16 | Schema v8: `Placement.centre_x_mm`, the pattern abscissa that goes to the placement angle (default the panel's centre) | The pieces of a cut panel start exactly where the whole did; estimating an angle per piece left gaps of several cm at the waist |
| 2026-11-16 | M12: `panel.cut` moves the outline edges and points of the smaller half to the new panel (same IDs) and keeps the larger half on the original panel | Seams, notches and pins on edges the cut does not touch survive without remapping; only the edges it crosses are split |
| 2026-11-16 | M13: pattern import as an action (`import.pattern`, reads inside the project folder) over an undoable command (`pattern.import`); actions take `&mut Workspace`; `Workspace::adopt` copies a user's file into `imports/` | One code path for the app, CLI and MCP, with the same path confinement as exports; the project keeps the files it was made from; agents can import structured pieces without a file |
| 2026-11-16 | Import parsers: `lopdf` (MIT; brings decompression and the decryption crates, not optional), `roxmltree` and `svgtypes` (MIT/Apache); our own DXF reader and writer | PDF objects, streams and filters are too much to reimplement; `usvg` would also resolve SVG but pulls in text shaping; DXF ASCII is simple and the AAMA subset is small |
| 2026-11-16 | Tiled PDFs are put back together by matching shapes across pages (the shift each page applies), not by a grid the user types | Works with no input for Rustle3D's own tiles and for the common "whole drawing on every page, clipped" layout; files that really split pieces across pages are refused with a clear error and `page` |
| 2026-11-17 | M14: the path tracer walks a BVH of our own in a compute shader; no hardware ray queries | wgpu 30 enables them only through an `unsafe` experimental-features token, and `unsafe` is denied; the compute BVH runs on every backend, including software Vulkan for tests |
| 2026-11-17 | Procedural studios (rectangular soft boxes that turn with the camera, a seamless backdrop) instead of shipped HDRI files | No asset licensing or download; soft boxes are sampled directly (next event estimation), so few samples are noisy; the look stays the same from every angle. The user's own HDRI can come later |
| 2026-11-17 | Our own denoiser (à-trous wavelet filter with SVGF's variance guidance) instead of Intel Open Image Denoise | OIDN is a C++ library with large binaries per platform and needs FFI (`unsafe`); a guided wavelet filter in WGSL runs on every GPU backend |
| 2026-11-17 | `Fabric::look()` (colour, sheen and its roughness, specular, translucency) replaces `Fabric::appearance()` | The 3D view, the photo and the glTF export read one table, so they agree on a fabric |
| 2026-11-17 | Whole-project operations (rename, delete, zip export/import) in `r3d_commands::projects`; `zip` crate (MIT) with deflate only (`flate2` on `miniz_oxide`, already in the tree) | Rename goes through `project.rename`, so the folder and the document agree; the start screen needs them on closed projects, outside any session; zip is the format users send each other, and the crate's other codecs (bzip2, zstd, lzma, AES) are not needed |
| 2026-11-17 | The app renders photos on a thread of its own on the views' GPU device; the 3D view draws the latest image (`ImageBlit`) | Cancelling and refining never block the UI; batches of about 40 ms of GPU time keep the views' frames smooth |
| 2026-11-17 | Zero-copy views on Windows: a shared D3D11 texture (NT handle) opened by wgpu's D3D12 device on Flutter's adapter; `render_view` becomes asynchronous; `windows` crate (MIT/Apache-2.0, the version wgpu-hal uses) in `r3d-ffi` | Views were read back through the CPU every frame (36 fps on the T-shirt demo); shared, they reach 60 fps. Blocking the UI thread on the GPU made it miss one vsync in 14 |
| 2026-11-17 | One GPU device for the whole app (`surface::ui_gpu`, made once on the UI's adapter), shared by every engine's views, the photo thread and the start screen's thumbnails | A device per thumbnail, made and dropped on worker threads (Vulkan, the default backend), hung the app on Windows: the GPU driver, detaching from an exiting thread, waited for another thread under the loader lock |
| 2026-11-17 | M17: a `Solver` trait over a compiled `Model`; the GPU backend in a crate of its own, `r3d-sim-gpu` (the roadmap said "in `r3d-sim`") | `r3d-sim` stays free of GPU dependencies (CLAUDE.md §2) and testable without a GPU; both backends share the model and the reference scenes |
| 2026-11-17 | M17: GPU XPBD over colour sets, no VBD spike | It meets the pass with room (T-shirt 5.4 ms, dress 5.3 ms per frame against 16.7 and 33; RTX 2080). The cost is per dispatch, not per particle, so VBD would only pay off by cutting substeps |
| 2026-11-17 | M17: the app drapes on the GPU (its one device), falls back to the CPU on failure or with `R3D_SOLVER=cpu`; headless `simulate` stays on the CPU | The app needs the speed; agents and the CLI need reproducible, cached drapes, and a GPU result depends on the GPU |
| 2026-11-17 | Flutter confirmed after a review of the GUI stack; Iced replaces Slint as plan B; Bevy and egui stay out | The 3D is ours (wgpu), Flutter only composites it: smooth on macOS (zero-copy), slower on Windows and WSL where views are read back, which zero-copy sharing fixes without a rewrite. Slint's licences are not permissive |
| 2026-11-18 | M22: sizes as grade rules on the base pattern (an offset per size on each point, and on a curve's handles when known), every size derived on demand (`Project::at_size`), not stored pieces per size | One geometry to edit; the rules are what DXF-AAMA/ASTM grading exchanges; a size cannot drift from its rules. Offsets per size rather than one step per point: real charts change their step (+4 then +6 cm), and a template drafted per size gives exact offsets |
| 2026-11-18 | M22: the drape uses the shown size through `Project::as_shown` inside `drape::build` and `Form::of_project`, so every caller (app, CLI, MCP, exports) drapes the same size | One choke point instead of each front-end grading the project |
| 2026-11-18 | M22: `rayon` in `r3d-commands` (already in the tree through `r3d-sim`): a graded template drafts every size in parallel; body fits are kept (`queries::fitted`, last 32) | Each size's draft fits a body (≈ 2.5 s): a shirt on 9 sizes took 26 s, now 6.5 s |
| 2026-11-18 | M26: history steps `history.list` and `history.go_to`, batch labels; the history names steps by command name (stable codes the UI translates) and undo/redo return the panels changed | One history for every front-end; labels stay translatable; the user sees what was undone |
| 2026-11-18 | M26: semantic copy as a query (`panel.copy` returns a clip, the pieces with their points, edges, notches, lines, pins and the seams between them, under the document's schema) and `panel.paste` (fresh IDs); the app passes clips through the system clipboard | Copy changes nothing, so it is not a command; a clip is plain JSON, so pasting into another project, or by an agent, needs no app state. A clip pastes only into the same schema version |
| 2026-11-18 | M26: progress and cancel through the command layer (`Progress`, `apply_with`, `run_with`); long engine calls asynchronous over the bridge, polled by the UI; the drape prepared on a copy of the document; body fits on a thread of their own, shared while in progress | Opening a project, new measurements and a graded template froze the app for seconds (a body fit is ≈ 2.5 s). Polling keeps the bridge simple (no streams) and every front-end reads the same reports |
| 2026-11-18 | M27: a geometric constraint solver of our own (`r3d_geom::solve`: damped minimum-norm Gauss–Newton, rank by Gram–Schmidt, Jacobian by central differences per equation) rather than a library | Sketches are small (tens of variables); the minimum-norm step is what makes a sketch move as little as possible, which general least-squares crates do not give; no new dependency |
| 2026-11-18 | M27: constraints as document entities (schema v11), solved after every step inside the step's transaction, the edited points held first | One undo step per edit, whatever the solver moves; agents and the app share the behaviour; a drawing can never be saved with a constraint broken |
| 2026-11-18 | M27: symmetric pieces as a half plus a derived reflection paired point for point and edge for edge (`Symmetry`), not a "cut on fold" flag read by the drape and the exports | The pattern stays a whole piece for every consumer (meshing, seams, exports, grading); pairing keeps the reflection's IDs, so seams on it survive edits of the drawn half |
| 2026-11-18 | M27: board snapping in the bridge (`r3d-ffi/src/snap.rs`), not a query | It depends on the zoom and on what is being drawn; agents send exact coordinates |
| 2026-11-18 | M28: a parametric piece grades itself by writing its points' grades (solved per size with that size's body), not by solving inside `Project::at_size` | Every consumer of sizes (board nest, exports, drape, mirrors, symmetric halves) already reads grades; `at_size` stays a cheap pure function; grade rules and formulas live in one model |
| 2026-11-18 | M28: formulas in our own small parser with dimensional analysis (length powers), not an expression crate | The grammar is tiny; units must be checked (a length plus a number is an error), which general evaluators do not do |
| 2026-11-18 | M29: a seam side is a path (a run of outline edges or an internal line, all or a stretch), schema v13; tools that reshape seams keep working on whole-edge seams and refuse the others | One model for side seams, partial seams, pockets on placement lines and pleats sewn to themselves; the drape pairs sides by resampling each into the same number of segments |
| 2026-11-18 | M29: one mesher for pieces with holes and sewn internal lines (`mesh_panel`): Ruppert refinement over the outline and holes, the lines inserted afterwards, triangles kept by centroid (flat slivers dropped) | A constraint that encloses nothing confuses `spade`'s inside/outside layers; inserting lines after refinement keeps triangle quality and the vertex order (the solver's results depend on it) |
| 2026-11-18 | M29: region booleans on a constrained Delaunay triangulation of both pieces (`add_constraint_and_split`), boundary edges between kept and dropped triangles chained into loops, not a polygon-clipping algorithm | Exact handling of crossings and shared stretches by the triangulation already in the tree; no new dependency |
| 2026-11-18 | M29: the board fills pieces from a constrained triangulation (`mesh::fill`) instead of ear clipping | Pieces may have holes; one triangulator for the board and the cloth |
| 2026-11-18 | M32: the Sew and Dimension tools pick on the board in the bridge (`r3d-ffi/src/sew.rs`, `corner_or_edge_near`) and build ordinary `seam.add` / `constraint.add` steps; a press a board tool does not hold falls through to Select's corner and edge drag | As for snapping: picking depends on the zoom; agents name edges, lines and stretches directly, so no new command was needed |
| 2026-11-18 | Licence changed from MIT to GPL-3.0-or-later before the repository goes public; dependencies must be GPL-3.0-compatible, permissive preferred (MPL-2.0, LGPL-3.0, GPL-3.0 with a justification) | Keeps Rustle3D and its derivatives free software, as Blender, Krita and Inkscape do: a fork or a closed product built on it must publish its changes. All dependencies so far (MIT, Apache-2.0, BSD, Zlib, OFL font, CC0 body) are compatible with GPLv3. Possible while every commit has a single author |
| 2026-11-18 | M23 (taken before M20, M21, M30, M24, M31): the avatar's skeleton is MakeHuman's default rig (CC0) reduced to 24 bones, its joints moved by the body's targets, in the body pack (v2); the motions are our own procedural ones (joint angles over a cycle), no motion capture | The rig and weights come with the body we already ship, under the same licence, and follow every measurement; CC-licensed motion capture sets have attribution or non-commercial strings attached, and three smooth cycles are short to write |
| 2026-11-18 | M23: a moving body is its rest-pose distance field queried through inverse linear blend skinning (each bone gives a starting point, one linear solve per step with the weights held), not a field rebuilt every frame nor a BVH over the posed mesh. A first version, the union of the bones' rigid pieces, was dropped | A rebuilt field costs 0.16 s a frame; a BVH would replace the constant-time query both backends are built on. The rigid pieces swept the shoulder and the chest up with a raised arm, where the blended skin does not go, and threw the cloth at 15 m/s; following the skin the views draw removed it |
| 2026-11-18 | M23: contacts are pre-stabilised (the push gives the cloth no more normal speed than the surface touched has) | Where linear blend skinning folds the skin (a raised arm's armpit), cloth caught in the fold was squeezed out at 10–20 m/s |
| 2026-11-18 | M23: the avatar's motion is app state (`Engine::set_motion`) and an argument of the headless `simulate` action, cached with the drape; not a document field | It does not change the garment, so it has no place in the undo history or the project file; agents reach it through `simulate`, and renders and exports read it from the cache |
| 2026-11-18 | M23: `r3d-sim` (as the table above already allowed) and `r3d-sim-gpu` depend on `r3d-geom` for `Rigid`, the rigid motion of a bone | The avatar crate and the solver share one type; `r3d-geom` is the pure-math crate both already sit on |
| 2026-11-18 | M30: the marker is nested by a raster bottom-left-fill of our own (`r3d_geom::nest`, Jakobs 1996, Burke et al. 2006, raster overlap after Bennell & Oliveira 2008), cells of 5 mm, largest piece first; computed on demand (`marker.compute`, `export.marker`), never stored | Deterministic, simple and fast enough for a garment in several sizes (a shirt in three sizes in well under a second), with no new dependency; a raster that marks every cell a piece touches can never overlap two pieces. No-fit polygons would be tighter (a cell of slack at most) and can replace it behind the same function. A marker is derived from the pattern and the fabric, so storing it would let it go stale |
| 2026-11-18 | M30: a garment's fabric details (weight, width, price per metre) on `Garment`, schema v14; the weight also sets the drape's areal density when given | The fabric as bought belongs to the garment like its fabric preset; one weight for the indicators and the cloth's mass, so they cannot disagree. Unset, the drape is unchanged (the presets' weights) |
| 2026-11-19 | M30: the tech pack's words are labels with stable keys and English defaults in `r3d-commands`, replaced by the front-end's own (`labels`); `r3d_io::techpack` lays out sections of tables, pictures, swatches, text and a drawing, and holds no words | A PDF is a file the user hands on, so it must be in their language, but core crates hold no translations (§7.7); the app already translates everything else, and agents get English |
| 2026-11-19 | M30: pictures for the tech pack are rendered by the front-ends (CLI and MCP with `r3d_mcp::render::techpack_views`, the app from its live 3D view) and passed in as decoded images; the action itself takes PNG files from the project folder | `r3d-commands` has no GPU; the app's pictures must show its running drape, which only the app holds. `png` (decode) and `miniz_oxide` (Flate for the PDF's images), both MIT/Apache and already in the tree, join `r3d-io` |
| 2026-11-19 | M24: fit maps and the tape measure are computed from positions in `r3d_commands::fit` (shared by the app's live drape on either backend and the cached headless one), not in the solver; agents read them through two read-only actions, `fit.report` and `fit.tape`, because only a workspace holds the cached drape (queries see the document alone); `drape::Latest` is the one loader of the cached drape (renders, 3D exports, fit) | One definition of strain, gap and ease whatever produced the positions; the GPU solver stays unchanged. A query would need the drape in the document, which belongs to the cache |
| 2026-11-19 | M24: a 3D mesh may carry per-vertex colours (`r3d_render::Mesh::colors`) for fit maps, drawn with the view's usual lighting; the colour scale is viridis (`r3d_render::fitmap`), fixed ranges (strain 0–20 %, gap 50–0 mm) | Folds stay readable under the colours; a perceptually uniform, colour-blind-safe scale; fixed ranges make maps of different garments comparable |
| 2026-11-19 | M30: the projector is a full-window Flutter route painting the pattern itself (`Engine::projector_drawing`, the same drawing as the 1:1 exports) at a calibrated scale stored as an app preference (`projector.json`); no OS full screen | Drawing in Flutter keeps it independent of the board's renderer and pan/zoom; the calibration belongs to the room, not the project. True full screen needs a window plugin (and `unsafe` on Windows): maximising the window on the projector does the job |
| 2026-11-19 | M21: a garment's print (built-in or an image file in the project's `textures/`, referenced by relative path) on `Garment`, a piece's shift and turn of it on `Panel`, schema v16; texture coordinates computed from the pattern along each piece's grainline (`Project::print_frame`), never stored | The print is a property of the fabric, laid on every piece by its grain as when cutting; storing coordinates per vertex would go stale with every edit, and images in the JSON would bloat it and its diffs |
| 2026-11-19 | M21: built-in prints drawn by our own code (`r3d_core::print::builtin`, in the garment's colour), no shipped image assets; prints decoded in `r3d-io` (`png`, and `zune-jpeg`, MIT/Apache-2.0/Zlib, pure Rust, for JPEG) and cached per front-end by `r3d_commands::prints::Prints` | No third-party image licences to track; built-ins follow the garment's colour. JPEG is what photos of fabric come as; `zune-jpeg` is the decoder the `image` crate itself uses now, without its other codecs |
| 2026-11-19 | M21: the photo tracer reads prints from the storage buffer it already had for the sheen table (one buffer: table, then a header and every print's texels), with manual bilinear filtering and no mip-maps | WebGPU guarantees eight storage buffers per shader stage and the tracer used all of them; binding arrays of textures are not portable. Samples accumulating per pixel filter the print |
| 2026-11-19 | M20: buttons and zippers as entity maps on `Project` (schema v17), omitted from the JSON when empty; a zipper's state is its open length from the top, not a slider position from the bottom | Projects without trims keep the same file (and the same drape cache key); a closed zipper stays closed when its sides change length |
| 2026-11-19 | M31: an imported body is a file in `avatars/` named in the document by relative path and fingerprint, with how to read it (scale, up axis, turn) and its arrangement points (schema v18); the file is read when the folder is opened or the body imported and kept in memory by fingerprint (`r3d_commands::bodies`), where commands, queries and `Form::of_project` find it | Commands and queries see only the document, and every front-end already opens projects through a `Workspace`; the fingerprint makes the drape cache follow the file and notices a file changed by hand. Passing the folder down to every `Form::of_project` caller (tests, renders, the app's form cache) would have touched them all for one case |
| 2026-11-26 | M41: `drape.diagnose` in `r3d_commands::diagnose`, a read-only action over `drape::Latest` like `fit.report`; `Drape` keeps each seam's range of sewn pairs | A report needs the cached drape, which only a workspace holds; seam gaps are named by seam, not by particle pair |
| 2026-11-26 | M40: PLY (ASCII and binary) and STL (binary and ASCII) bodies read by `r3d-io`'s `model` too, STL corners welded by exact position | Both formats are small and fully specified; no crate needed |
| 2026-11-19 | M31: model files read by `r3d-io` itself (`model`: OBJ, glTF/GLB with embedded buffers; our own base64), `glam` added to `r3d-io` for node transforms; no `gltf`/`tobj` crates | The subset a body needs is small (positions, triangles, node transforms); glam is already in the tree; compressed glTF (Draco, meshopt) is refused with a message rather than pulling in decoders |
| 2026-11-19 | M31: the distance field of an open or non-manifold body signs nodes near the bad edges by generalized winding numbers (Barill et al.'s hierarchy, `r3d_geom::winding`), elsewhere by pseudo-normals as before | Exact and unchanged for closed bodies; robust near holes at the cost of a few thousand winding numbers; both solvers read the same field |
| 2026-11-19 | M15: the game-ready export is a separate action (`export.game`, `r3d_commands::game`) writing its own glTF scene type (`r3d_io::game`), not options of `export.gltf`; mesh simplification and texture baking are our own code in `r3d-geom` (`decimate`, `bake`), no new dependency; LODs are separate nodes named `_LODn`, not `MSFT_lod` | The plain export stays one node per piece for 3D tools; a game wants one mesh per garment, an atlas, tangents, a skin and animations. Half-edge collapses keep the full mesh's vertices, so the levels share its atlas and skin weights and the normal maps bake without ray casting. `MSFT_lod` is not read by the engines' importers we target and the validator cannot check it |
| 2026-11-19 | M20: closures are a constraint kind of their own in the solvers (`SimScene::closures`), a stiff distance whose target shrinks at 0.5 m/s, not seams; opening or closing one rebuilds the scene and carries the cloth over | Seams close on the sewing ramp and are stiff after it: a button fastened later snapped shut at ≈ 50 m/s. The rebuild path already carries pattern edits into a running drape, so no solver needs constraints added or removed at run time |
| 2026-11-19 | The guided flow's step bar and goal banner removed (spec 11, `docs/ux.md` §1); the tool rail in four sections set apart by a line: drawing, measuring, sewing, trims (Dart and Pleat in the sewing section, Hole in the drawing one) | Maintainer's change request: every step was already one click away (Body, the rail, the inspector, Simulate), so the bar took room without helping; grouping the rail by purpose makes it easier to read. Learn and the first tour stay the help |
| 2026-11-19 | Captured motions (spec 22): BVH files are read and retargeted in `r3d-avatar` (`bvh`, `captured`), not in `r3d-io`; `Motion` gains a `Captured` variant holding the retargeted clip (no longer `Copy`); the files live in the project's `motions/` folder and are named by path (in the app's motion state, `simulate`'s `motion_file` and the drape cache, with a hash of the file), not in the document | Retargeting needs the skeleton, which lives in `r3d-avatar`, and the parser is 200 lines with no dependency; the motion stays app state as decided for M23, and a file in the project folder, like imported bodies, travels with the project and is reachable from the CLI, MCP and the app alike |
| 2026-11-19 | The motion library (spec 22) is a folder of BVH files in the app's library folder (`r3d_avatar::motion_library`, reached from `r3d-ffi`'s `library` API like body presets), not a command; playing a library motion copies it into the project | The library is the user's, outside any document, as body presets are; a project that holds its own copy renders, caches and travels without the library, and the CLI and agents keep working on project files |
| 2026-11-19 | Contributions under a Contributor License Agreement (`CLA.md`: a licence grant to the maintainer and his successors, including other licences than the GPL, with a commitment that every version including a contribution stays available under GPL-3.0-or-later or another OSI licence), signed once by a PR comment through CLA Assistant Lite (`.github/workflows/cla.yml`, signatures on the `cla-signatures` branch); no DCO | The maintainer keeps the right to relicense (commercial licences, app stores) to fund the project; a licence grant rather than an assignment, which French law makes heavy and which cannot cover moral rights; the open-source commitment answers contributors' fear of a project closed later |
| 2026-11-19 | Releases are built by `.github/workflows/release.yml` on a pushed tag `v<version>` (checked against `Cargo.toml`) and published as a GitHub Release with `SHA256SUMS.txt`, unsigned for now; each package holds `LICENSE`, `THIRD_PARTY_NOTICES.md`, the Rust crates' licences (`cargo-about`, `about.toml`) and Flutter's, and a README with the source address of the tag | The site's download buttons point at the release assets; the GPL and the MIT/Apache/BSD dependencies ask for the licence texts to travel with the binaries; signing (Apple notarization, a Windows certificate) comes later and changes neither the names nor the layout |
| 2026-11-20 | Release packages: a universal `.dmg` on macOS (the app holds `r3d`, built for both architectures and joined with `lipo`, and the licences, signed again ad hoc), an Inno Setup installer on Windows x64 (`packaging/windows/rustle3d.iss`, per-user by default, Start menu entry, uninstaller), the `.tar.gz` on Linux; the site reads the new names | A zip did not say which processor it was for, and the app unzipped from it on macOS was hard to install; Inno Setup is a free build tool whose licence allows distributing the installers, and an unsigned MSIX cannot be installed at all. An installer does not remove the warning about an unsigned app: notarization (Apple) and a code-signing certificate (Windows) do |
| 2026-11-20 | No demo script per milestone: `demos/` removed, the maintainer tests each milestone against its spec; the scripts some tests read moved to `tests/fixtures` of their crate; `ROADMAP.md` keeps one line per closed milestone, its history is in git | The demos were no longer run and drifted from the commands; the roadmap had grown into a log |
| 2026-11-20 | Seamly2D/Valentina patterns are imported by running their own drafting steps (`r3d_io::import::val`) for a body, not by turning them into our constraints; the garment keeps the pattern's and measurement file's text in `project.json` (`Garment.source`, schema v19) and is graded, or drafted again (`garment.redraft`), by running it for other bodies and moving each outline point by the difference | Their tools (shoulder points, trued darts, curve intersections) have no simple counterpart in a constraint solver, and a partial translation would draft wrong pieces; the text in the document keeps commands pure (no file reads) and survives the files being moved; moving points by differences keeps the edits made in Rustle3D, as template grading does |
| 2026-11-24 | M36: the avatar's skin follows its bones by dual quaternion skinning (Kavan et al. 2008), in the views, the drape's collisions (CPU and GPU) and the arms' clearance; the game export keeps linear blend skinning (glTF's) for the garment it unposes | Linear blend skinning folded the armpit with the arms above about 40° and shrank bent joints; a blended rigid motion keeps their volume, and the inverse skinning becomes a rigid inverse (distances and normals carried exactly instead of to first order) |
| 2026-11-25 | M36: built-in motions place the feet first and the legs reach them by closed-form two-bone inverse kinematics (`r3d_avatar::legs`); the pelvis's height is the legs' limit over the cycle, eroded by parabolas (bounded upward bend) and smoothed, never above rest; the walk is a treadmill walk whose swing follows published joint angle curves bent to its foot placements (`r3d_avatar::gait`) | Joint angles alone (forward kinematics) with the body dropped onto its lowest sole made the feet slide and the body hop where the lowest point changed; planting the feet makes no-slip a construction, and a pelvis path bounded in curvature cannot hop. A smoothed lower envelope keeps the pelvis as high as the legs allow, so the knees are not bent more than needed |
| 2026-11-25 | A garment can be taken out of the drape (`Garment::draped`, schema v26, `garment.set_draped`): `drape::build` works on the project with that garment's pieces unplaced | Unplaced pieces were already left out of the drape everywhere; unplacing them in a copy for the build keeps every other path unchanged and keeps the placements for when the garment comes back |
| 2026-11-25 | M37: an imported body moves with our skeleton, not its own: `r3d_avatar::rig` places the built-in body's skeleton (fitted to its measurements) on it, by its file's joints (found by name, as for captured motions) or its arrangement points, and splits the result into a rest skeleton (its lengths, our directions) and the stance it stands in; `drape::Rig` moves the body by each bone's motion from the stance; the game export writes joints in the stance (with rotations) and skins the imported body; the closest-point weight transfer moved to `r3d-commands::skinning` | The motions (procedural, captured, inverse kinematics, arm clearance) are written for our skeleton in its A pose; a rest skeleton with our directions lets every one play unchanged, and keeping the stance as the bind pose leaves the body and the export as the file has them. Retargeting each motion onto a file's own skeleton would have needed every motion rewritten per rig and nothing for bodies without one |
