# Roadmap

From an empty window to v0.1. Every milestone ends with something you can
demo, a green CI, updated docs and a demo script (CLI or MCP).

Estimates assume one developer working with AI assistance, in focused weeks.
They are rough, and the order can change as we learn (see "Firm vs. flexible"
in `CLAUDE.md`).

The order is chosen so that **agents can drive the app early**: the command
layer and the headless MCP server come before the GUI, so most later features
can be tested through MCP as soon as they exist.

| # | Milestone | Estimate | Demo |
|---|---|---|---|
| M0 | Bootstrap | done | `cargo build && cargo test` green, planning docs |
| M1 | Document + commands + CLI ✅ | 1.5–2 wk | CLI script builds panels, undo/redo, save/reopen |
| M2 | Geometry, 2D export, headless MCP ✅ | 2 wk | An agent drafts a skirt pattern via MCP and exports a tiled PDF |
| M3 | Flutter + Rust spike, design system, 2D editor | 4–5 wk | Draw and edit panels in a polished UI (**visual gate**) |
| M4 | Simulation v1 ✅ | 3 wk | A pinned panel drapes in real time in the 3D view |
| M5 | Avatar, collisions, arrangement ✅ | 3 wk | Panels arranged around the avatar fall onto it |
| M6 | Sewing + synced views ✅ (UI tools → M8) | 3 wk | A skirt made from scratch, sewn and draped |
| M7 | Materials, self-collision, grab/pin ✅ (T-shirt → M10) | 2–3 wk | The same T-shirt in jersey vs denim; pull it with the mouse |
| M8 | 3D → 2D editing tools ✅ | 3 wk | Lengthen a hem and take in a seam directly in 3D; the pattern follows |
| M10 | Templates, guided flow, polish → **v0.1** ✅ (released 2026-11-16) | 3–4 wk | A first-time user makes a T-shirt in < 10 minutes |
| M9 | glTF export, attached MCP ✅ | 2 wk | An agent edits the garment live in the open app; the result opens in a game engine |
| M11 | Pattern drafting tools ✅ | 3–4 wk | Draw a bodice with the curve tool, set the distance between two points, gather a skirt into a waistband |
| M12 | Drafting tools 2: curve family, cut ✅ | 2–3 wk | Draw a curved piece in one go, cut a bodice along a curved princess line into two pieces sewn together |
| M13 | Pattern interchange: DXF, SVG and PDF ✅ | 2–3 wk | A pattern drawn elsewhere is imported from DXF-AAMA, SVG or PDF, edited, sewn and exported back |
| M14 | Photorealistic rendering ✅ | 4–6 wk | A path-traced studio image of the draped garment |
| M16 | Zero-copy views on Windows ✅ and Linux | 1–2 wk | The garment drapes as smoothly on Windows and Linux as on macOS |
| M17 | GPU cloth solver ✅ | 3–4 wk | A T-shirt drapes at 60 fps; a dress stays interactive |
| M18 | Trousers and shirt templates ✅ | 1–2 wk | A first-time user makes trousers and a shirt from templates |
| M19 | Layers and multiple garments (tucked in: open) | 3–4 wk | A shirt tucked into trousers, each in its own fabric and colour |
| M22 | Grading and sizes ✅ | 2–3 wk | A shirt graded from 36 to 52, each size draped on a matching avatar |
| M25 | Maximized views ✅ | < 1 wk | The board fills the window, the drape pauses; one click brings both views back |
| M26 | History, editing comfort, feedback on long steps ✅ | 2–3 wk | Jump back ten steps from the history panel; paste two sewn pieces with their seam; a graded template shows its progress |
| M27 | Sketching 2: constraints and symmetry ✅ | 4–5 wk | Draw a half bodice against a symmetry axis with fixed dimensions; change one, the shape follows |
| M28 | Parametric patterns ✅ | 2–3 wk | A skirt whose waist is `waist / 4 + 10 mm` redrafts itself on another body |
| M29 | Construction details | 3–4 wk | A skirt with darts, a pleat and a patch pocket sewn on an internal line |
| M20 | Trims: buttons and zippers | 2–3 wk | A shirt that buttons and trousers with a fly zipper, draped closed |
| M21 | Textures and prints | 2–3 wk | A striped shirt whose stripes follow the grainline on every piece |
| M30 | Production: marker, yardage, tech pack, projector | 3–4 wk | A graded shirt nested on a 150 cm fabric, its yardage and cost, its tech pack PDF |
| M24 | Fit maps and virtual tape measure | 2 wk | Tight spots show in colour; the ease at the chest is read on the drape |
| M31 | Imported avatars | 2–3 wk | A body scan in OBJ/glTF is checked, the pieces placed around it and draped |
| M23 | Animated avatars | 3–4 wk | The avatar walks and the garment follows |
| M15 | Game-ready export | 3–4 wk | LODs + baked wrinkle normal maps + skinning, imported in Godot/Unity/Unreal |

M10 came before M9: the first-time user test, not the export, decided
what v0.1 needed. After v0.1 the order is M9, M11, M12, M13, M14, M16,
M17–M24, M15. M12 came out of the M11 test: the drafting tools settle
before imported patterns need them. M16 came before M15: the GUI stack
review of 2026-11-17 kept Flutter (architecture §7.1) and found the views
slow wherever they are read back. M17–M24 came before M15 on 2026-11-17:
the points found in use (templates, several garments, textures) and the
garment features they lead to come first, and the export then ships them
with skinning to an animated avatar. The GPU solver leads because every
later milestone adds cloth to simulate (physics.md §7: the T-shirt is at
70 ms per frame against a 33 ms target).

The maintainer's review of 2026-11-18 put the editor first: M25–M29
come before M20, M21, M24, M23 and M15. Drafting comfort (views,
history, constraints) is what is felt every day, and every later
feature is drawn with these tools. M25 is a quick win; M26 comes before
M27 because constraints and formulas add steps that must undo cleanly;
M28 builds on M27's dimensions (a formula is a computed dimension); M29
gives the details that make a garment look real. M30 groups what a
small brand needs to produce. M19's open point (close layers, a tucked
in shirt) is hard physics: it is taken up after M29.

---

## M1 — Document + commands + CLI
- `r3d-core`: IDs, `IdMap`, project/panel/point/edge model, JSON with
  `schema_version`, migration framework, golden tests.
- `r3d-commands`: `CommandSpec`, `Transaction` with recorded changes,
  history with merge keys, batch, first commands (`project.new`,
  `panel.create_rect`, `panel.create_polygon`, `point.move`, `panel.delete`,
  `panel.rename`), first queries.
- `r3d-cli`: `new`, `run`, `query`, `schema`.
- `xtask gen-docs` → `docs/commands.md`.
- **Demo**: `./demos/m1/demo.sh` (A-line skirt pattern built by a script,
  with undo/redo, an atomic batch, queries and a structured error).
- Delivered: 9 commands, 4 queries, steps (`history.undo`, `history.redo`,
  `batch`), integrity checks, golden files, `cargo xtask gen-docs`.

## M2 — Geometry, 2D export, headless MCP
- `r3d-geom`: cubic Bézier (split, length, parameter at length, tangent),
  mirror transform, outward offset for seam allowances.
- Schema v2 (first real migration): notches, internal lines, seam
  allowance, linked mirror copies.
- Commands: `edge.split`, `point.remove`, `notch.*`, `internal_line.*`,
  `panel.set_grain`, `panel.set_seam_allowance`, `panel.mirror`,
  `panel.unlink`; query `measure.contour_length`.
- `r3d-io`: SVG and tiled PDF (A4/Letter/single page) with the 100 mm test
  square, from one shared drawing.
- `Workspace` + action registry (`export.svg`, `export.pdf`) with confined
  file writes; CLI `export` and `mcp`.
- `r3d-mcp`: stdio server generated from the registries, autosave,
  structured tool errors.
- **Demo**: `./demos/m2/demo.sh` (script-built A-line skirt, an MCP session,
  exports).
- Moved to M4: panel triangulation, needed only by the solver. Moved to M3:
  nearest point on a curve, needed only for UI snapping.

## M3 — Flutter + Rust spike, design system, 2D editor
- **Spike (≈1 week, first)**: Flutter UI + Rust wgpu views shown as a
  zero-copy texture (architecture §7.1), with Slint as plan B. Validate
  against the pass criteria with the maintainer, then record the decision.
- Repo layout: `app/` (Flutter, Dart) next to `crates/`; `r3d-ffi` bridges
  Dart and Rust with `flutter_rust_bridge` over the command/query/event
  protocol.
- Design system in Flutter: tokens, light/dark themes, fonts, icon set,
  core widgets, motion; i18n message catalog (`en`, ARB files).
- `r3d-render`: board renderer (on-screen texture + offscreen PNG) → MCP
  `render_board`.
- Editor: selection tool, rectangle/polygon/Bézier tools, snapping, inline
  dimensions with `Tab`, undo history panel, command palette, status bar hints.
- **Visual gate**: screenshots of the start screen, editor and inspector in
  both themes, reviewed before going further.
- **Demo**: draw a skirt by hand; an agent renders the board to PNG.

## M4–M7 — Simulation, avatar, sewing, materials ✅ (closed 2026-11-15)

Built together, because each needed the others to be shown. Demo:
`./demos/m4/demo.sh` (CLI: body, sewn skirt template, drapes in cotton and
denim, a pin, renders), then the same live in the app.

Done:
- `r3d-geom`: constrained Delaunay meshing of panels at a target spacing,
  boundaries sampled edge by edge so seam sides match.
- `r3d-sim`: XPBD small steps (30 substeps), strain-based membrane,
  dihedral bending, seams with a sewing ramp, pins, gravity, ground,
  friction; avatar contacts through a narrow-band SDF (pseudo-normal
  sign); particle self-collision; grab; contacts in parallel; deterministic
  for any thread count; reference scenes and `criterion` benchmarks.
- Avatar: MakeHuman CC0 body fitted to measurements, size presets and the
  user's named presets (Body panel).
- Document (schema v4–v5): seams, panel placements, garment fabric and
  colour, pins — all through commands, undoable, available to agents.
- Sim bridge (`r3d_commands::drape`): placed panels start around the body
  at the garment's own width; drape cache keyed by a document fingerprint.
- App: simulation thread; Simulate / Pause / Restart; fabric swatches;
  per-piece rendering with fabric looks; click picks the piece in 3D (the
  selection is shared with the pattern view); drag pulls the cloth;
  ⌥-click pins it; pins listed and removable.
- CLI / MCP: `simulate` action, `r3d simulate`, `r3d render --view
  board|3d`, MCP `render_board` / `render_3d` (PNG).
- One template source (now the parametric recipes of M10).

Measured and dropped: per-colour parallel constraint solving (slower at
garment sizes, `docs/physics.md` §6). The planned triple buffer is a
mutex-guarded snapshot (enough at these sizes).

Moved to later milestones:
- To M8: sewing and arranging in the UI (seam tool with mismatch badges
  and direction arrows, placement around the body, "Arrange all"), live
  re-simulation after pattern edits (positions carried over), camera
  presets. They are what makes the 2D ↔ 3D loop fluid.
- To M10 (templates, polish): a T-shirt template and the jersey vs denim
  T-shirt demo; the material library and an advanced material inspector.
- Performance targets of `docs/physics.md` §7 (measured ≈ 2× off): a
  better-converging solver or the GPU backend, after v0.1.
- OBJ/glTF avatar import (with the winding-number SDF sign): with M9.

## M8 — 3D → 2D editing tools ✅ (closed 2026-11-15)

Demo: `./demos/m8/demo.sh` (shorten 30 mm and take in the waist from a
script; drape, pattern and PDF follow); in the app, the same with
⇧-drag in 3D.

Done: live re-simulation after any edit (positions carried over); Length
(⇧-drag a hem, `edge.lengthen`, the whole hemline moves); Fit (⇧-drag a
seam, `seam.take_in`, tapered near an end, symmetric across linked
mirrors); Pick (panel and edge from either view, exact ray / triangle
hit; the edge card in the inspector); Draw on garment (D, internal line
on the pattern); the Sew tool (S, click two edges; direction chosen);
seams drawn on the board; "Arrange around the body" (`garment.arrange`).
A skirt made from scratch with these sews, arranges and drapes (test).

Left for later: picking points and notches in 3D; internal lines and the
selected edge drawn on the 3D garment; per-panel placement in the UI and
dragging panels in 3D (M10); camera presets (M10); mismatch badges and
seam direction arrows on the board (M10).

## M9 — glTF export, attached MCP ✅
- glTF 2.0/GLB garment export with pattern UVs and `KHR_materials_sheen`;
  OBJ; glTF validator in CI; checked imports in Blender and Godot.
- Attached mode: app-hosted MCP over a local socket, `r3d mcp --attach`,
  agent indicator, disconnect button.
- **Demo**: the agent changes the skirt length while the user watches it
  re-drape live.

Built (2026-11-16), demo `./demos/m9/demo.sh`:
- `export.gltf` (GLB) and `export.obj`, `r3d export --format glb|obj
  [--avatar]`, and an Export menu in the app (pattern SVG/PDF, garment
  GLB/OBJ as the 3D view shows it). One named node per piece, pattern
  UVs, PBR + `KHR_materials_sheen`; every template passes the Khronos
  glTF Validator (`scripts/validate-exports.sh`, in `check.sh` and CI).
- Attached MCP: Agents menu in the app (opt-in), loopback TCP + token,
  `r3d mcp --attach` relay, live refresh, undoable, "Agent connected"
  and Disconnect. End-to-end tests: Rust (host + MCP session, CLI
  relay) and the app (an agent creates a piece; the inspector counts it).
- Still to check by hand: imports in Blender and Godot. `render_3d` in
  attached mode shows the cached drape, not the live one.

## M10 — Templates, guided flow, polish → v0.1 ✅ (released 2026-11-16)
- Parametric templates (skirts, T-shirt, dress, tote bag), guided flow step
  bar, Learn panel, first-launch tour.
- Usability pass and the 10-minute test with first-time users.
- Autosave/recovery, release builds for macOS, Windows and Linux.
- **Demo**: a first-time user makes a T-shirt in under 10 minutes.

Built (2026-11-16), demo `./demos/m10/demo.sh`:
- `template.apply` / `template.list`: T-shirt, dress, straight skirt,
  A-line skirt and tote bag drafted from sizes (the avatar's by default),
  sewn, placed and given a fabric through ordinary commands; each drapes
  in a test with no cloth inside the body.
- Placements (schema v6) wrap a panel around the body or an arm (sleeves;
  the dress form finds its upper arms), or lay it flat (the tote bag,
  hung from two pins). Gravity ramps in while seams close; tops start
  above the shoulders; skirts hang from the measured waist; the dress
  form's bust is smoothed.
- App: start screen (template cards with rendered thumbnails, sizes sheet
  per body, blank project, recent projects in Documents/Rustle3D, saved
  after every change), live step bar Measurements → Pattern → Sew → Fabric
  → Drape (goal, action, highlighted tool), Learn panel (task cards whose
  "Show me" points at where things are), first-launch tour.
- From the M8 leftovers: style lines and the selected edge drawn on the 3D
  garment (also in agents' renders); seam direction chevrons and red
  mismatched seams on the board; camera views (keys 1–4); per-piece
  placement in the inspector; ⌘-drag moves a piece around the body.
- Linux and Windows runners, with a copied-pixels view path (shared
  textures are macOS-only); `scripts/release.sh` and a manual "Release
  builds" workflow (unsigned packages).

First-time user test (2026-11-16): conclusive, partly passed; the app
stays simple. Fixed from it before the release:
- Learn's "Show me" points at where things happen instead of changing
  the project; keys in bold, shown as the platform's (Ctrl on Linux and
  Windows).
- The UI in French, with a language picker (English by default).
- Start screen in sections: recent projects with a picture of their
  pattern, "Open…" from disk, all projects, the projects folder, the
  version and rustle3d.com.
- Inspector: an edge's length typed or with a slider (`edge.set_length`);
  a seam's two sides made the same length (`seam.match_length`); typed
  slider values; no overflow in French.
- Pattern: drag a corner or an edge on the board (`edge.move`); trackpad
  pinch and two-finger zoom.
- Drape: cloth no longer cuts into the bust (edge middles kept out of the
  body); Coulomb friction on the body; fabric shear an order below
  tension and a firmer cotton (the back of the dress no longer crumples);
  the dress's side seam 1 cm towards the back; closed seams move as one
  in self-collision.
Version 0.1.0; packages from the manual "Release builds" workflow
(unsigned).
Known: XPBD creep lets a skirt slide a few cm below the waist before it
settles (physics.md §7 convergence).

---

## M11 — Pattern drafting tools ✅
- The tool rail's waiting tools: **Polygon** (P, click the corners of a
  new piece), **Curve** (C, draw a curved edge or bend a straight one by
  its handles), **Notch** (N).
- Point selection in 2D and 3D (corners, points on edges, notches),
  carried to the inspector.
- **Distance between two points** of two edges (e.g. two notches, a
  shoulder and a side point): measured, and typed to move one of them.
  To define: a construction curve or guide line between them that the
  pattern follows, and whether the distance stays fixed afterwards (a
  dimension) or is a one-off move.
- **Gathers**: a seam whose longer side is gathered evenly onto the
  shorter one (a gathered skirt into a waistband, a sleeve cap), with its
  gathering ratio; possibly drawn as a gathering curve derived from the
  construction curve above. Needs its own seam pairing in the solver
  (spec to write).
- **Demo**: draw a bodice with the curve tool, set the distance between
  two notches, gather a skirt into a waistband and drape it.

Built (2026-11-16), demo `./demos/m11/demo.sh`:
- Polygon (P), Curve (C, handles dragged as one undo step, Straighten)
  and Notch (N) on the board; corners picked with Select, ⇧-click a
  second to see and set their distance (`point.set_distance`).
- Gathers (schema v7): `seam.set_gathered`, `seam.list` gives the
  gathering ratio; the Gathered switch in the edge card; the board does
  not flag a gathered seam. The drape already pairs both sides point
  for point, so a 2:1 gathered skirt drapes with its seams closed.
- Not yet: points on edges and notches as distance targets, picking
  points in 3D, construction lines that keep a distance (dimensions), a
  gathering curve; UI tests for dragging handles and for the distance
  field (their commands are tested).

## M12 — Drafting tools 2: curve family, cut ✅
From the M11 test (2026-11-16): what Curve does today is right but
belongs in a family of edit tools; the Curve tool itself should draw.
- **Tool groups** on the rail: a tool can hold sub-tools (a small
  flyout; the last one used stays on the button).
- **Curve** (C) draws a piece like Polygon, but each edge can be curved
  as it is drawn (click for a corner, drag for a smooth point with its
  handles); the first point or Enter closes the piece.
- **Edit curve** group: today's bend-by-handles, Straighten, and
  convert an edge to a curve (or back); later: add/remove a point on a
  curve, smooth/corner point.
- **Cut** (K): draw a straight or curved line across a piece; on
  confirm the piece becomes two pieces sewn together along the cut
  (one command, one undo step). Notches, internal lines, grain line and
  fabric are carried to the side they fall on; existing seams on the
  cut edges are split with them. Needs `panel.split` in r3d-commands
  (spec to write) and a drape check that the sewn halves drape like the
  original piece.
- **Demo**: draw a curved piece with Curve, cut a bodice along a curved
  princess line, drape both halves sewn together.

Built (2026-11-16), demo `./demos/m12/demo.sh`:
- Tool groups on the rail (a small corner marks them; a click opens the
  menu, the button shows the last tool used). **Edit curves**: Bend (B,
  M11's curve editing), Straighten (L), Add point (A, `edge.split`).
- **Curve** (C) draws a piece: click for a corner, drag for a smooth
  point; `panel.create_path` (straight and cubic edges, any direction).
- **Cut** (K): a straight or curved line across a piece, Enter cuts;
  `panel.cut` (spec 16) keeps the larger piece's ID, moves the other's
  edges and points to a new panel, sews them along the cut, splits a
  sewn edge with its partner, carries notches, lines and pins, and moves
  the new piece 20 mm away. Schema v8: `Placement.centre_x_mm`, so the
  pieces of a placed panel start exactly where the whole did (an A-line
  skirt cut along a princess line drapes like the whole one).
- `panel.translate` now moves the panel's pins too.
- Not yet: editing handles of a drawn point afterwards, smooth/corner
  toggle, removing a point from the board, cutting a piece and its linked
  mirror copy together (refused for now: unlink first).
- Found: on the Men L body an A-line skirt in denim slides off the hips,
  cut or not (the waist grip is too weak; the women's bodies hold). To
  look at with the solver's convergence (physics.md §7).

## M13 — Pattern interchange: DXF, SVG and PDF ✅
- **DXF-AAMA / ASTM D6673** import and export (open standards): piece
  outlines, internal lines, notches, grain line, piece names and sizes;
  units checked on import.
- **SVG and PDF import** of piece outlines from vector drawings (SVG and
  tiled PDF export exist since M2), with the scale given or read from the
  document; pages of a tiled PDF put back together.
- Round-trip tests on reference files we draw ourselves.
- **Demo**: a pattern drawn elsewhere is imported from DXF-AAMA, SVG or
  PDF, edited, sewn, draped and exported back.

Built (2026-11-16), demo `./demos/m13/demo.sh`:
- **Export DXF-AAMA** (`export.dxf`, `r3d export --format dxf`, Export →
  "Pattern, DXF-AAMA"): one block per piece, cutting line, sewing line,
  turn and curve points, notches, grainline, internal lines, piece name.
- **Import** (`import.pattern` over the undoable `pattern.import`;
  `r3d import`; Import in the top bar): DXF-AAMA and plain DXF, SVG, PDF.
  Units from the file (or asked in the app when it says nothing),
  sewing line and seam allowance, notches, grainline, darts and internal
  lines, names; polylines fitted back to editable curves (Schneider
  1990, `r3d_geom::fit`); tiled PDFs put back together by matching the
  pages' shapes. The file is kept in the project's `imports/`.
- Grainlines are drawn inside the piece in every export (a bag's handle
  opening pushed them out).
- Tests: every template through SVG, PDF (one page, A4 tiles) and DXF
  and back; hand-drawn references (AAMA in inches, Inkscape-style SVG).
- Not yet: sizes and grading (AAMA RUL files), seams in the exchange
  (DXF-AAMA has none), PDFs whose pages hold different parts of the
  pattern, binary DXF, pieces with holes, the tiling preview.

## M14 — Photorealistic rendering ✅
- Progressive GPU path tracer in `r3d-render` (wgpu compute, hardware ray
  queries when available), physically based fabric materials (sheen, fuzz,
  thickness), HDRI studio lighting, denoising; renders in the background and
  refines in the 3D view; `render_photo` for CLI/MCP.

Built (2026-11-17), demo `./demos/m14/demo.sh`; algorithms and papers in
`docs/rendering.md`:
- `r3d_render::photo`: path tracing with next event estimation on soft
  boxes and MIS, over a SAH bounding volume hierarchy built on the CPU and
  walked in a compute shader; Phong-tessellated cloth and body (round
  folds and silhouettes), shadow rays off the shading surface; a layered
  fabric BSDF (Charlie sheen, GGX specular, diffuse reflection and
  transmission) from `Fabric::look()`, now shared with the 3D view and the
  glTF export (which gains each fabric's sheen roughness); deterministic
  sampling.
- Three procedural studios (Studio, Soft, Dramatic): soft boxes that turn
  with the camera and a seamless backdrop, instead of HDRI files.
- Denoising with an à-trous wavelet filter guided by normals, depth and
  variance; Khronos PBR Neutral tone mapping, so a fabric's colour comes
  out as chosen.
- Headless: MCP `render_photo` (quality, lighting, camera angle, framing,
  samples, bounces, denoise) and `r3d render --view photo`.
- App: Photo button over the 3D view (quality Quick/Good/Best, lighting,
  size up to 4K, advanced settings folded); the photo refines in the 3D
  view on a thread of its own, with progress, Stop, Save image… (PNG) and
  Close.
- Measured without a GPU (software Vulkan, 16 threads): a 480 × 360 Good
  photo of the T-shirt in 15 s; the 1920 × 1080 Good target (< 1 min) is
  to measure on the reference Mac.
- Not yet: hardware ray queries (an unsafe, experimental wgpu feature),
  the user's own HDRI, textures and weave normal maps, cloth thickness at
  the edges.

## M16 — Zero-copy views on Windows and Linux
macOS shares the views' texture with Flutter (IOSurface); Linux renders
into a texture of its own and reads it back through the CPU every frame,
which makes the views slow there.
- Windows ✅ (2026-11-17): the runner creates a shared D3D11 texture on
  Flutter's adapter and registers its NT handle as a GPU surface; wgpu
  opens it as a D3D12 resource on a device on the same adapter (LUID).
  `render_view` is asynchronous, so the UI thread never waits for the
  GPU. Release build, T-shirt demo draping, RTX 2080 at 60 Hz: UI 59.9
  fps (build 0.7 ms median), a 3D frame every 16.7 ms; read back, the
  same scene ran at 36 fps.
- Linux: a Vulkan texture exported as a dma-buf, imported by the GTK
  runner as an EGL image behind an `FlTextureGL`.
- `unsafe` kept to the sharing code in `r3d-ffi`, as for the IOSurface;
  the readback stays as the fallback (WSL, software GPUs).
- Pass: in release builds, 60 fps with the 3D view draping and UI
  animations running, input latency under one frame, on Windows and
  native Linux.

## M17 — GPU cloth solver ✅ (2026-11-17)
The CPU solver misses its budget beyond a skirt (physics.md §7), and
M19–M23 all add cloth or motion.
- Done: `r3d-sim-gpu`, XPBD on wgpu compute behind the `Solver` trait
  (a crate of its own, so `r3d-sim` stays free of GPU dependencies);
  constraints in colour sets, one dispatch per colour; body, edge and
  self contacts gathered per particle, the self-collision candidates
  from a spatial hash rebuilt on the GPU every frame. The app drapes on
  its shared GPU device and falls back to the CPU; headless `simulate`
  stays on the CPU (deterministic, cached).
- Measured (RTX 2080, 16-thread CPU, `cargo bench -p r3d-sim-gpu`):
  T-shirt 5.4 ms per frame (CPU 29.7), dress 5.3 ms (CPU 36.7). In the
  app, sharing the GPU with the views: 7.8 ms, views at 60 fps.
- XPBD kept, no VBD spike: the GPU cost is per dispatch, not per
  particle (the dress at 2.3k → 6.5k particles: 5.4 → 6.2 ms), so VBD
  would only pay off by cutting substeps, which the pass does not need.
  It stays the lead when M19–M23 need more.
- Every reference scene and starter garment passes the same checks on
  both backends; the GPU drape ends within a few mm of the CPU's.
- Not measured: Apple M-series (Metal), Linux (Vulkan).

Plan as written:
- A second backend behind the `Solver` trait, in `r3d-sim` with wgpu
  compute (no GUI dependency): GPU XPBD over the existing colour sets,
  or Vertex Block Descent [physics.md 12] if a spike shows it converges
  better at the same cost. The choice is made on measurements.
- Collisions with the avatar SDF and self-collision on the GPU.
- The CPU backend stays: deterministic regression tests, and machines
  without a usable GPU.
- Pass: T-shirt ≥ 60 fps and dress ≥ 30 fps on the reference machines;
  the same reference scenes pass the regression checks on both backends.

## M18 — Trousers and shirt templates ✅ (2026-11-17)
Found by an agent drafting trousers and a shirt over MCP (2026-11-17):
trousers placed around the body with sewn inseams already separate onto
the legs.
- Done: `shirt` (two fronts, back, long sleeves, cuffs, stand collar) and
  `trousers` (front and back per leg) templates, sized from the avatar;
  on the start screen. New placements `left_leg` and `right_leg`: around
  the leg below the crotch, around that side of the hips above it
  (physics.md, starting positions). Sleeves and cuffs start as closed
  tubes around the arm.
- Every seam closed and no cloth in the body: women's M and men's L on
  the CPU, women's M on the GPU.
- Not yet: buttons on the shirt's front and cuffs (M20; the fronts are
  sewn shut), a waistband and fly (M20), darts.

Plan as written:
- Trousers and shirt (collar, cuffs, sleeves) templates in the guided
  flow, sized from the avatar's measurements.

## M19 — Layers and multiple garments (2026-11-17: tucked in still open)
Today one project holds one garment: a shirt and trousers share one
fabric and colour.
- Done: several garments per project, each with its own fabric and
  colour (drape, 3D view, photos, glTF/OBJ materials); a layer order
  (`garment.set_layer`), the outer garment starting over the inner one;
  templates added to a project become garments of their own; the
  garments list in the inspector; schema v9 (spec 17).
- Open: close layers. A shirt worn over trousers drapes with closed
  seams but they show through each other where they touch, and a shirt
  cannot be tucked in: self-collision keeps layers about 2 cm apart
  (particle radius 0.4 × spacing). Three start poses were measured and
  failed (physics.md, "Several garments"). Next: thin cloth–cloth
  contacts (vertex–triangle) between garments, CPU and GPU.

Plan as written:
- Several garments per project, each with its own fabric and colour;
  a layer order so the outer garment drapes over the inner one (a shirt
  tucked in or worn out).

## M20 — Trims: buttons and zippers
- Buttons and buttonholes, zippers, placed on the pattern and simulated
  as closures; open and close them in 3D.

## M21 — Textures and prints
- UVs from the pattern pieces, aligned with the grainline.
- Users add their own textures (prints, weaves) and the app offers a
  default set; they show in the 3D view, the photo renderer and the
  exports.

## M22 — Grading and sizes ✅ (2026-11-18)
Done ahead of M20–M21 (maintainer, 2026-11-17), with the range up to 52
instead of 46.
- Done: a size range on the project (`grading.set_sizes`, or the
  built-in European charts with `grading.use_chart`: women 32–56, men
  44–60), each size with a body built from the avatar; grade rules on
  points (`point.set_grade`: a step per size, or offsets in some sizes),
  curves, internal lines, pins and mirror copies following; every size
  derived from the base (`Project::at_size`), the board, the drape and
  the exports reading it; schema v10 (spec 18).
- Template garments (added before or after the sizes are set) are
  drafted for every size's body:
  a shirt graded 36–52 is, in every size, exactly the shirt drafted for
  that body, its seams matching (`grading.check`), the sizes nested on
  the board without touching the next piece.
- `grading.show_size` drapes a size on its own body (app, CLI, MCP);
  clicks on the draped garment come back to the base pattern.
- The graded nest on the board, in SVG and PDF (a colour per size, a
  legend) and DXF-AAMA (a block per piece and size, `Size:`); the app's
  pattern exports of a graded project are the nest; `r3d export --sizes`.
- The app: a Sizes section in the inspector (chart, range, base, size
  shown in 3D), a grade step on a selected point.
- Open: the shirt template drapes with folds at the sides from about
  size 48 (the shirt drafted directly for that body does too: the
  template, not the grading); notches graded on their own; an ASTM rule
  table (`.rul`) and grading read from imported files. A graded
  template takes about 6 s to add on 9 sizes (one body fit per size, in
  parallel).

Plan as written:
- A size range on the pattern: grade rules move the key points from
  one size to the next (e.g. waist +10 mm, armhole lower by 5 mm), and
  every size is generated from the base size.
- Drape each size on an avatar with matching measurements; export the
  graded nest (DXF-AAMA, PDF).

## M25 — Maximized views ✅ (2026-11-18)
- A button on the board and on the 3D view makes it take the space of
  both views (the inspector stays); a button, the same one or a
  shortcut, brings back the default layout.
- While the 3D view is hidden, a running drape pauses, and resumes when
  the view comes back (the user's own pause is kept).
- **Demo**: maximize the board while a shirt drapes, the drape pauses;
  restore, it goes on.

Built (2026-11-18), UI only (no command; the layout is not saved):
- The button at the top right of each view and **M** over a view; the
  board keeps its pan and zoom when its view is recreated (also on a
  window resize, which used to fit it again).
- A drape running while the board is maximized (an agent's too) is
  paused and resumes on restore; Space or the guide's Drape brings the
  3D view back. Tested by `app/integration_test/views_test.dart`.

## M26 — History, editing comfort, feedback on long steps ✅ (2026-11-18)
Spec 08 already plans most of it.
- Undo/Redo labels naming the action ("Undo Move point") in the Edit
  menu and their tooltips; steps grouped the way the user sees them (a
  drag, a tool's whole gesture, a template added: one step each).
- A history panel: the list of steps, a click jumps back or forward to
  any of them.
- Undo and redo restore the selection (and select what changed), so the
  user sees what was undone.
- Semantic copy and paste: pieces pasted with the seams between them,
  their internal lines, notches, pins, grade rules and mirror links (in
  the same project or another one). Needs a `panel.paste` command.
- **Feedback on long steps** (maintainer, 2026-11-18): the app never
  looks frozen. Long steps run off the UI thread and show their
  progress: opening a project, adding a template (about 6 s graded on 9
  sizes, one body fit per size: a bar per size), setting a size chart,
  importing a pattern, exporting (graded nests, glTF), fitting a new
  body, starting the drape. A spinner when the length is unknown, a
  progress bar with what is being done when it is known, a cancel
  button where stopping is safe. The core reports progress through the
  command layer, so the CLI and MCP see it too.
- **Demo**: draw, sew and move pieces, jump back ten steps from the
  panel, redo; copy two sewn pieces into a new project; add the shirt
  template to a project graded 36–52 and watch its sizes being drafted.

Built (2026-11-18), demo `./demos/m26/demo.sh`:
- History: `history.list` and `history.go_to` steps (MCP tools too),
  batch labels; undo, redo and go-to return the panels they changed and
  the app selects them (what changed, rather than the selection before:
  the selection is not in the history). The app has no menu bar: the
  undo and redo arrows name the action in their tooltips, and a clock
  button lists the steps, the latest first; a click goes there. A
  gesture is one step (release seals the history).
- `panel.copy` (a query: a clip in JSON) and `panel.paste`; ⌘C/⌘V in
  the app through the system clipboard, so pieces paste into another
  project. A piece pasted where another already starts around the body
  is left unplaced.
- `Progress` through the command layer (`apply_with`, `run_with`): a
  graded template reports each size and stops between sizes; the
  headless drape reports frames. CLI: one stderr line in a terminal.
  MCP: progress notifications on a progress token, cancellation honoured.
  App: long calls are asynchronous, a card (after 300 ms) shows a bar or
  an indeterminate bar, with Stop for templates and size ranges; the
  body is fitted on a thread of its own while the 3D view keeps the old
  one, and fits in progress are shared (`queries::fitted`).
- Not covered by the Flutter integration tests yet (they run on macOS):
  the history list, ⌘C/⌘V and the progress card were checked by hand on
  Windows.

## M27 — Sketching 2: constraints and symmetry ✅ (2026-11-18)
From the maintainer (2026-11-18): drafting lacks ease. General CAD ideas
(constraints, dimensions, symmetry), in our own UI and terms.
- **Constraints** on points and edges: coincident, horizontal, vertical,
  parallel, perpendicular, tangent (smooth joint between curves), equal
  length, fixed point, point on an edge. Shown on the board, removable.
- **Dimensions**: lengths, distances and angles that stay fixed (the
  open point of M11); editing one value redraws the piece.
- A geometric constraint solver (least squares with Newton steps,
  under-/over-constrained states reported, never a crash), from the
  published literature, cited in the code and the docs. Every solve is
  one command, one undo step.
- **Symmetry axis** while drawing: draw half a piece against an axis,
  the other half follows (unfolded on the fold, or kept as a linked
  mirror).
- **Snapping and inference**: to points, midpoints, edges, the axis,
  horizontal/vertical, with a visible hint.
- **Construction lines** (not cut, not sewn) to draft against.
- New primitives: arc, **arc sector** (circle skirts, collars, given a
  waist length and a skirt length), ellipse.
- Editing what M12 left open: handles of a drawn point afterwards,
  smooth/corner toggle, removing a point.
- **Demo**: draw a half bodice against an axis with fixed dimensions;
  change the bust dimension, the piece and its other half follow; draw
  a circle skirt from an arc sector.

Built (2026-11-18), demo `./demos/m27/demo.sh`, spec 19:
- Constraints and dimensions (`constraint.add` / `remove` /
  `set_value` / `list`; schema v11) solved after every step in the same
  undo step by our own damped minimum-norm Gauss–Newton solver
  (`r3d_geom::solve`), the edited points held first; conflicts refused
  with the constraints concerned; degrees of freedom and redundant
  constraints reported.
- Symmetric pieces: `panel.unfold`, or `symmetric` when drawing a polygon
  or a path (the closing edge is the axis); the reflected half follows,
  keeps its IDs (seams on it stay) and its notches; `panel.unlink`
  separates the halves. Linked mirror copies as before.
- Construction lines (`guide.*`), drawn dashed across the board; snapping
  in the app to corners, middles, edges, lines, axes and level with the
  previous point, with a hint; a dragged corner snaps too.
- `panel.create_ellipse`, `panel.create_sector` (circle skirts from the
  waist and the length), `edge.set_arc`; `point.set_smooth` (smooth and
  corner points); Delete removes a picked corner.
- The app: ⇧-click picks a second corner or edge; constraint chips and a
  Constraints section in the inspector (values typed), signs and
  dimensions over the board; the Half piece option, the Construction
  line tool (G), the Shapes group (rectangle, ellipse E, circle skirt),
  the Arc tool.
- Open: constraints on internal lines, notches and construction lines;
  dragging a dimension on the board; dragging a point's handles with
  Select (Bend does it, and a smooth point keeps them aligned).

## M28 — Parametric patterns ✅ (2026-11-18)
- A dimension (M27) can be a **formula** of the body's measurements and
  named variables: `bust / 4 + 20 mm`. Formulas are checked (unknown
  name, cycle, bad unit) and errors reported with codes.
- The pattern redrafts when the body changes. With sizes (M22), a
  formula is evaluated with each size's measurements, so a parametric
  piece grades itself; grade rules stay for pieces without formulas
  (imported patterns). Formulas do not replace grade rules: they are
  kept only if they fit that model simply.
- Agents get the formulas through commands ("make this pattern to my
  measurements").
- Then, if worth it: import Seamly2D/Valentina patterns and measurement
  files (open XML formats, formulas kept). Implemented from the format,
  no code taken from those GPL projects.
- **Demo**: a skirt with `waist / 4 + 10 mm` redrafts itself on another
  body and in every size.

Built (2026-11-18), demo `./demos/m28/demo.sh`, spec 20:
- Formulas on dimensions (`constraint.set_formula`, or `formula` when
  adding one) and named variables (`variable.set` / `remove` / `list`;
  schema v12): arithmetic with units checked as it is computed, body
  measurements height, bust, waist, hips; errors with codes
  (`formula.syntax`, `unknown_name`, `cycle`, `unit`, `no_body`, `domain`,
  `range`, `bad_name`).
- Redrafted after every step (a new body, a changed variable); with a
  size range, every size is drafted with its own body and the points the
  formulas move get their grades from it, so the graded nest, exports and
  per-size drape follow; grade rules stay for the rest.
- The app: a formula button on each dimension, a Variables section.
- Open: more body measurements (neck, shoulder, arm…); Seamly2D/Valentina
  import (left for later: the formulas work, the format is a separate
  piece of work); formulas typed directly in a value field.

## M29 — Construction details
- Seams on part of an edge and on a chain of edges, with an adjustable
  **ease** (spec 02's open point).
- **Seams on internal lines**: a patch pocket sewn on its placement
  line, a pin tuck.
- **Darts** (drawn, folded and sewn in one tool), **pleats** and knife or
  box pleating.
- **Holes** in a piece, **booleans** between two pieces (union,
  difference).
- Later in the milestone if the solver allows: elastic and shirring,
  visible topstitching, interfacing (local stiffening), quilting and
  padding.
- **Demo**: a skirt with darts, a pleat and a patch pocket sewn on an
  internal line, draped.

## M30 — Production: marker, yardage, tech pack, projector
- **Indicators per piece**: area, fabric mass; totals per garment.
- **Marker (nesting)**: the pieces of one or several sizes laid out
  automatically on a given fabric width, grainline respected, to reduce
  waste; yardage and cost from the fabric's price. Algorithm from the
  open literature.
- **Projector export**: the pattern at 1:1 on a projector over the
  fabric (full-screen calibrated view, high contrast, mirror option),
  without printing.
- **Tech pack**: a PDF with views of the garment, the measurement table,
  the bill of materials (fabrics, trims), colourways and sewing notes.
- **Demo**: a graded shirt nested on a 150 cm fabric with its yardage
  and cost; its tech pack; the back piece projected at 1:1.

## M31 — Imported avatars
- Import a body in OBJ, glTF or GLB (a scan, a game character); the
  user owns its licence.
- A check of the collision mesh (closed, scale, orientation, holes)
  shown before draping; the arrangement points placed with help and
  checked before the simulation starts.
- Later: automatic fit of the arrangement points to the body.
- **Demo**: a scanned body is checked, a shirt placed around it and
  draped.

## M23 — Animated avatars
- A skeleton for the avatar and a few motions (walk, turn, raise arms);
  the cloth follows the moving body. Animation assets under a
  redistributable licence only (CLAUDE.md §1).

## M24 — Fit maps and virtual tape measure
- Stress and strain shown in colour on the draped garment, to find
  where it is tight or loose.
- A tape measure on the drape: circumferences of the garment and ease
  (the gap between body and garment) read where the user clicks.

## M15 — Game-ready export
- Decimated LODs, wrinkle detail baked into normal maps, back faces or
  thickness; skinning to the avatar skeleton (M23); presets for Godot,
  Unity and Unreal.

## Later (architecture kept ready, not scheduled)
Material calibration from measurements (a home protocol: a strip
hanging over an edge for bending, a weight and a ruler for stretch; a
community fabric library under CC0; only data whose licence allows
commercial use) · turntable video of the photo render (a runway walk
after M23) · step-by-step sewing instructions generated from the seams
(needs a sewing order) · OpenUSD
export · finer cloth mesh with a strain limit (smaller folds;
physics.md §7) · picking points and notches in 3D · signed packages.
