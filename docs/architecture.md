# Rustle3D architecture

Status: living document, last updated 2026-11-18 (M29).

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
| `r3d-sim-gpu` | `r3d-sim`, `wgpu`, `bytemuck`, `pollster`, `thiserror` | core, commands, render, GUI |
| `r3d-io` | `r3d-core`, `r3d-geom`, `pdf-writer`, `lopdf`, `roxmltree`, `svgtypes`, `serde_json`, `thiserror` | sim, GUI, GPU; no file I/O (takes and returns bytes) |
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
  regenerates `docs/commands.md` from the registry.

## 3. Document model (`r3d-core`)

```
Project                                        (✓ = implemented in M1)
 ├─ name, display_unit, next_id                ✓
 ├─ garments: IdMap<GarmentId, Garment>        ✓
 ├─ panels: IdMap<PanelId, Panel>              ✓ (name, garment, contour, grain angle)
 │    Panel { name, contour: Vec<EdgeId>, internal_lines, notches,
 │            symmetry: Option<SymmetryLink>, material: MaterialId,
 │            grain_angle, particle_spacing, placement: Placement3d }
 ├─ points:  IdMap<PointId, Point2>            (shared by edges, mm)
 ├─ edges:   IdMap<EdgeId, Edge>               (line or cubic Bézier between two PointIds)
 ├─ seams:   IdMap<SeamId, Seam>               (two sides: edges or an internal line, a stretch, a direction; ease)
 ├─ materials: IdMap<MaterialId, FabricMaterial>
 ├─ avatar:  Option<AvatarRef>                 (path inside the project folder + landmarks)
 ├─ grading: Option<Grading>                   ✓ (M22: sizes with body differences, base, shown)
 ├─ constraints: IdMap<ConstraintId, Constraint> ✓ (M27: geometric constraints and dimensions)
 ├─ guides: IdMap<GuideId, Guide>              ✓ (M27: construction lines)
 ├─ variables: name → formula                  ✓ (M28: named variables for formulas)
 ├─ arrangement_points: from the avatar, editable
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
 ├─ avatars/            imported avatar meshes (copied in, referenced relatively)
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
  the drape keeps going: no reset, no jump.
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
| 2026-11-18 | Licence changed from MIT to GPL-3.0-or-later before the repository goes public; dependencies must be GPL-3.0-compatible, permissive preferred (MPL-2.0, LGPL-3.0, GPL-3.0 with a justification) | Keeps Rustle3D and its derivatives free software, as Blender, Krita and Inkscape do: a fork or a closed product built on it must publish its changes. All dependencies so far (MIT, Apache-2.0, BSD, Zlib, OFL font, CC0 body) are compatible with GPLv3. Possible while every commit has a single author |
