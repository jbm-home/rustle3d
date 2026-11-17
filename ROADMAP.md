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
| M17 | GPU cloth solver | 3–4 wk | A T-shirt drapes at 60 fps; a dress stays interactive |
| M18 | Trousers and shirt templates | 1–2 wk | A first-time user makes trousers and a shirt from templates |
| M19 | Layers and multiple garments | 3–4 wk | A shirt tucked into trousers, each in its own fabric and colour |
| M20 | Trims: buttons and zippers | 2–3 wk | A shirt that buttons and trousers with a fly zipper, draped closed |
| M21 | Textures and prints | 2–3 wk | A striped shirt whose stripes follow the grainline on every piece |
| M22 | Grading and sizes | 2–3 wk | A shirt graded from 36 to 46, each size draped on a matching avatar |
| M23 | Animated avatars | 3–4 wk | The avatar walks and the garment follows |
| M24 | Fit maps | 1–2 wk | Tight spots on a garment show in colour on the avatar |
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

## M17 — GPU cloth solver
The CPU solver misses its budget beyond a skirt (physics.md §7), and
M19–M23 all add cloth or motion.
- A second backend behind the `Solver` trait, in `r3d-sim` with wgpu
  compute (no GUI dependency): GPU XPBD over the existing colour sets,
  or Vertex Block Descent [physics.md 12] if a spike shows it converges
  better at the same cost. The choice is made on measurements.
- Collisions with the avatar SDF and self-collision on the GPU.
- The CPU backend stays: deterministic regression tests, and machines
  without a usable GPU.
- Pass: T-shirt ≥ 60 fps and dress ≥ 30 fps on the reference machines;
  the same reference scenes pass the regression checks on both backends.

## M18 — Trousers and shirt templates
Found by an agent drafting trousers and a shirt over MCP (2026-11-17):
trousers placed around the body with sewn inseams already separate onto
the legs.
- Trousers and shirt (collar, cuffs, sleeves) templates in the guided
  flow, sized from the avatar's measurements.

## M19 — Layers and multiple garments
Today one project holds one garment: a shirt and trousers share one
fabric and colour.
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

## M22 — Grading and sizes
- A size range on the pattern: grade rules move the key points from
  one size to the next (e.g. waist +10 mm, armhole lower by 5 mm), and
  every size is generated from the base size.
- Drape each size on an avatar with matching measurements; export the
  graded nest (DXF-AAMA, PDF).

## M23 — Animated avatars
- A skeleton for the avatar and a few motions (walk, turn, raise arms);
  the cloth follows the moving body. Animation assets under a
  redistributable licence only (CLAUDE.md §1).

## M24 — Fit maps
- Stress and strain shown in colour on the draped garment, to find
  where it is tight or loose.

## M15 — Game-ready export
- Decimated LODs, wrinkle detail baked into normal maps, back faces or
  thickness; skinning to the avatar skeleton (M23); presets for Godot,
  Unity and Unreal.

## Later (architecture kept ready, not scheduled)
Material calibration from measurements · OpenUSD
export · finer cloth mesh with a strain limit (smaller folds;
physics.md §7) · picking points and notches in 3D · signed packages ·
a loading indicator while a project opens (opening runs on the UI
thread and blocks it for a moment; move it off and show progress).
