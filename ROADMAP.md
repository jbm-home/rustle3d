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
| M19 | Layers and multiple garments ✅ | 3–4 wk | A shirt tucked into trousers, each in its own fabric and colour |
| M22 | Grading and sizes ✅ | 2–3 wk | A shirt graded from 36 to 52, each size draped on a matching avatar |
| M25 | Maximized views ✅ | < 1 wk | The board fills the window, the drape pauses; one click brings both views back |
| M26 | History, editing comfort, feedback on long steps ✅ | 2–3 wk | Jump back ten steps from the history panel; paste two sewn pieces with their seam; a graded template shows its progress |
| M27 | Sketching 2: constraints and symmetry ✅ | 4–5 wk | Draw a half bodice against a symmetry axis with fixed dimensions; change one, the shape follows |
| M28 | Parametric patterns ✅ | 2–3 wk | A skirt whose waist is `waist / 4 + 10 mm` redrafts itself on another body |
| M29 | Construction details ✅ | 3–4 wk | A skirt with darts, a pleat and a patch pocket sewn on an internal line |
| M32 | Editor finishing ✅ | 1–2 wk | A patch pocket sewn on its line and a dimension drawn on the board, all with the mouse |
| M23 | Animated avatars ✅ | 3–4 wk | The avatar walks and the garment follows |
| M30 | Production: marker, yardage, tech pack, projector ✅ | 3–4 wk | A graded shirt nested on a 150 cm fabric, its yardage and cost, its tech pack PDF |
| M24 | Fit maps and virtual tape measure ✅ | 2 wk | Tight spots show in colour; the ease at the chest is read on the drape |
| M20 | Trims: buttons and zippers ✅ | 2–3 wk | A shirt that buttons and trousers with a fly zipper, draped closed |
| M21 | Textures and prints ✅ | 2–3 wk | A striped shirt whose stripes follow the grainline on every piece |
| M31 | Imported avatars ✅ | 2–3 wk | A body scan in OBJ/glTF is checked, the pieces placed around it and draped |
| M15 | Game-ready export ✅ | 3–4 wk | LODs + baked wrinkle normal maps + skinning, imported in Godot/Unity/Unreal |

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
in shirt) is hard physics: it is taken up after M32. M32 (2026-11-18)
brings to the app what M27–M29 left to agents and scripts: the commands
exist, the mouse cannot reach them yet. M23 came next (maintainer,
2026-11-18): it could be done autonomously, the skeleton and weights
shipping with the body we already use (CC0) and the motions being our
own. M30 followed on 2026-11-18–19, before M20 and M21, for the same
reason: production needs no new physics or assets. M24 came next
(2026-11-19), also before M20 and M21: it reads the drape we already
have. M21 followed the same night (2026-11-19): prints need no new
physics either, and every later render and export shows them. M20
closed the same night (2026-11-19), the first of these to touch both
solvers. M31 closed after it (2026-11-19): imported bodies reuse the
distance field both solvers read, and stand still. M15 closed next
(2026-11-19): it exports what all of these build, skinned to M23's
skeleton. M19's close layers closed last (2026-11-19): thin contacts
between garments, in both solvers.

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
  sewn shut), a waistband and fly (M20), darts. (2026-11-19: the
  buttons and the fly came with M20; a waistband and darts are still
  open.)

Plan as written:
- Trousers and shirt (collar, cuffs, sleeves) templates in the guided
  flow, sized from the avatar's measurements.

## M19 — Layers and multiple garments ✅ (2026-11-19)
Today one project holds one garment: a shirt and trousers share one
fabric and colour.
- Done: several garments per project, each with its own fabric and
  colour (drape, 3D view, photos, glTF/OBJ materials); a layer order
  (`garment.set_layer`), the outer garment starting over the inner one;
  templates added to a project become garments of their own; the
  garments list in the inspector; schema v9 (spec 17).
- Done (2026-11-19): close layers. Garments touch through thin
  point–triangle contacts (`r3d_sim::layers`, CPU and GPU) instead of
  particle repulsion, which kept them 2 cm apart: a shirt worn under
  trousers is tucked into them, and a T-shirt over trousers hides them.
  Sides come from the layer order, candidates are gathered again as the
  cloth moves, open seams are bridged, and cloth resting on the body
  lets the garment over it take the push (physics.md, Several garments).
- Done: the trousers' back waist starts behind the body instead of in
  front of it (it was dragged round through a shirt tucked in, which
  happened to cinch the waist). Placed right, the trousers settle a
  little lower; the template's default length is now the waist's
  height less 4 cm (it was 0.565 of the stature: the men's hem lay on
  the floor).
- 2026-11-19 (maintainer: a T-shirt over trousers sits wrong): the
  T-shirt started inside loose trousers at the hips; fixed (physics.md,
  Several garments).
- Open: edge–edge contacts between garments, and point–triangle contacts
  within a garment (its own folds stay a particle diameter apart); on
  the GPU a shirt and trousers step in 13.9 ms (6.2 ms without contacts
  between garments), a hash of its own for self-collision is the lever.
- **Demo**: `./demos/m19/demo.sh`: trousers and a shirt worn over them,
  then the shirt tucked in (garment.set_layer), draped and photographed
  from the front and the back.

Plan as written:
- Several garments per project, each with its own fabric and colour;
  a layer order so the outer garment drapes over the inner one (a shirt
  tucked in or worn out).

## M20 — Trims: buttons and zippers ✅ (2026-11-19)
- Buttons and buttonholes, zippers, placed on the pattern and simulated
  as closures; open and close them in 3D.
- **Demo**: `./demos/m20/demo.sh`: a men's shirt that buttons down the
  front (7 buttons) and at the cuffs, and trousers with an 18 cm fly
  zipper, draped closed and rendered (board, 3D); then the two top
  buttons undone and the fly opened, draped again without the
  closures that were let go; the shirt's tech pack lists the buttons.
- Delivered (spec 26): schema v17, buttons (a point on a piece, a
  diameter, a buttonhole on the same or another piece, buttoned or
  not) and zippers (two sides given as a seam's, an open length from
  the top). Commands `button.add`, `button.set`, `button.set_closed`,
  `button.remove`, `zipper.add`, `zipper.set_open`, `zipper.remove`,
  query `trim.list`; a stretch is sewn or zipped, never both. Board:
  buttons at their size, slits with bar tacks, zippers as a brass band
  (solid closed, dashed open). Tech pack: trims in the bill of
  materials. Drape: a closure constraint on both solvers, a stiff
  distance whose target shrinks at 0.5 m/s (a button fastened while the
  garment hangs closes without a jolt); buttons and buttonholes made
  mesh vertices; the layers around a buttoned button may lie on each
  other (no self-collision there); opening or closing rebuilds the
  scene and carries the cloth over. 3D view, renders and tech pack
  pictures: buttons as dark discs, zippers as a brass band. Templates:
  the shirt's fronts with a 15 mm band, buttoned (left over right on a
  men's body), cuffs with a 2 cm lap and a button; trousers with a fly
  zipper above the crotch seam. App: Button and Zipper tools in the
  rail's Trims menu; a piece's card buttons, unbuttons (all), opens a
  zipper with a slider, removes trims; integration test
  `trims_test.dart`.
- Open: trims in photos (the path tracer) and in glTF/OBJ exports;
  buttons kept when pieces are copied and pasted, or mirrored; the
  Zipper tool on part of an edge (agents can); a button's own colour
  and shape (shank, four holes); vertical buttonholes drawn along the
  band; a waistband with a button on the trousers; snaps, hooks and
  elastic. The overlapping bands of a shirt may pass through each
  other near a button (self-collision is off there, see physics.md).

## M21 — Textures and prints ✅ (2026-11-19)
- UVs from the pattern pieces, aligned with the grainline.
- Users add their own textures (prints, weaves) and the app offers a
  default set; they show in the 3D view, the photo renderer and the
  exports.
- **Demo**: `./demos/m21/demo.sh`: a navy-striped shirt (2 cm repeat)
  whose stripes run along every piece's grainline, draped, rendered to
  PNG (board, 3D view, photo), exported to glTF (validated) and OBJ with
  its MTL and image; then a check image of its own, 30 cm wide, turned
  45° on one front.
- Delivered (spec 25): schema v16, a garment's print (built-in
  `stripes`, `gingham`, `dots`, `twill` drawn by our code in the
  garment's colour, or a PNG/JPEG copied into the project's `textures/`
  folder), its repeat (mm) and angle; a piece's shift and turn of it.
  Texture coordinates computed from the pattern in the print's frame
  turned to each piece's grainline (`Project::print_frame`), so the
  print has its real size on every piece and in every size. Commands
  `garment.set_print`, `garment.remove_print`,
  `panel.set_print_placement`; `r3d texture FILE|builtin`. Shown in the
  3D view (mip-mapped, sheen tinted by the print's average), the board
  (pieces filled with their print), photos (albedo read from the print
  in the tracer, the denoiser guided by it), glTF (image embedded,
  repeating sampler; Khronos validator clean) and OBJ (MTL with
  `map_Kd`, images beside it). App: Print in the garment's card (none,
  the four built-ins as swatches, your image), repeat width and angle,
  a preview tile; Print on this piece (shift, turn) with a piece
  selected. JPEG decoding with `zune-jpeg` (MIT/Apache-2.0/Zlib).
- Open: placing pieces on the print by dragging them over it, stripe and
  check matching in the marker (pieces are nested as on a plain
  fabric), weave relief (normal and roughness maps), transparency, a
  different print on the inside of the cloth, a user library of prints
  shared across projects.

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
- 2026-11-19 (maintainer: graded trousers fell from size 52, arms came
  out of the sleeves): the avatar fit collapsed past size 50 (a women's
  52, waist 104 cm, came out at 69 cm) and every size from 44 up was
  slimmer than asked (a pull to the neutral body outweighed the
  residuals). The fit now backtracks, falls back to steepest descent and
  barely regularises: every chart size within 1 cm but the women's 56
  (≈ 5 cm short: past it the arms meet the torso). In the app, a new
  body (size shown, measurements) starts the drape again around it: the
  cloth carried over onto a larger body left arms out of their sleeves.
  Trousers' default length is the waist's height less 4 cm.
- Done (2026-11-19): the shirt, T-shirt and dress take shoulders, neck
  and armhole depth from the avatar (`r3d_avatar::fit::upper_body`: the
  skeleton's shoulder joint and neck base, the armpit where the torso's
  girth drops) with ease, the shoulders widened by 0.05 mm per mm of
  chest over the neutral body (the body model's shoulders do not widen
  with its weight); their hem takes the hips (a women's 52 shirt opened
  at the front). Drafted from the chest, a 52's shoulders were 26 cm
  each side for 19 on the body.
- Open: a women's 52 shirt parts a little below its last button over
  the belly (no darts); trousers hold at the waist by friction only (a 52 settles
  under the belly, its hem on the floor); notches graded on their own; an ASTM rule
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
- Open: more body measurements (neck, shoulder, arm…); formulas typed
  directly in a value field.
- Seamly2D/Valentina import, built 2026-11-20 (spec 28, demo
  `./demos/m28/seamly.sh`): `.val`/`.sm2d` patterns are run with their
  measurement file (`.vit`, `.vst`, `.smis`, `.smms`) through our own
  reading of their drafting steps and formulas (`r3d_io::import::val`),
  the pieces exact with seam allowance, notches, grainline, internal
  paths and fold lines; the garment keeps the pattern (schema v19), is
  graded in every size by running it for each size's body, and is
  drafted again for another body keeping the edits (`garment.redraft`);
  a measurement file gives the avatar, variables and, multisize, the
  size range (`import.measurements`). The app's Import takes these files
  and a garment card offers Draft again for the avatar. Open: the steps
  `parallelCurve`, `graduatedCurve`, `ellipticalArcWithLength` and the
  union of pieces; seam allowances that change along the outline.

## M29 — Construction details ✅ (2026-11-18)
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

Built (2026-11-18), demo `./demos/m29/demo.sh`, spec 21:
- Seam sides as paths (schema v13): a whole edge, a run of outline edges
  or an internal line, all or a stretch; seam ease; overlapping stretches
  refused. The drape cuts every seam side into as many segments as its
  partner, spread over its stretches; sewn internal lines and holes are
  part of the mesh (`r3d_geom::mesh::mesh_panel`).
- `dart.add` (drawn, folded, sewn), `pleat.add` (knife or box, stitched
  down along internal lines), `panel.add_hole` / `remove_hole`,
  `panel.boolean` (union, difference; CDT-based region booleans in
  `r3d_geom::boolean`, the result fitted to curves).
- The app: Details tools (Dart, Pleat, Hole), Join or cut in the
  inspector, a seam's Ease in the edge card.
- Open: elastic and shirring, topstitching, interfacing, quilting and
  padding (need solver work); partial and line seams from the app's Sew
  tool; holes in mirror copies and symmetric pieces; seam allowance
  inside holes.

## M32 — Editor finishing ✅ (2026-11-18)
From the maintainer (2026-11-18): what M27–M29 built must be drawable
in the app, not only through commands. Mostly UI work on existing
commands; in this order:
1. **Sew tool**: seams on part of an edge, on a run of outline edges and
   on an internal line (M29's open point), picked with the mouse.
2. **Dimensions drawn on the board**: pick two points or an edge and
   pull a dimension out, its value typed in place (M27's open point).
3. Dragging a point's handles with Select (M27); formulas typed directly
   in a value field (M28).
4. If time allows: constraints on internal lines, notches and
   construction lines (M27); holes in mirror copies and symmetric
   pieces, seam allowance inside holes (M29).
- **Demo**: a skirt whose patch pocket is sewn on its placement line
  and whose waist dimension is drawn on the board, all with the mouse;
  an integration test drives the same steps.

Built (2026-11-18), demo `./demos/m32/demo.sh` (the three integration
tests below, in the real app):
- Step 1: the Sew tool picks a whole edge or internal line (click), a
  stretch of one (drag along it, the ends caught on corners, notches and
  other seams' ends) or a run of edges (⌘-clicks in any order, put in
  contour order); the track under the pointer lights up
  (`r3d_ffi::sew`, `app/integration_test/sewing_test.dart`).
- Step 2: the Dimension tool (T): an edge's length, two corners'
  distance (direct, width or height from where it is pulled out) or two
  edges' angle, measured, then its value typed where it was placed
  (`app/integration_test/dimension_test.dart`).
- Step 3: any dimension field takes a formula in place of a number (on
  the board and in the inspector); with Select, a picked curved edge or
  a corner between curves shows its handles, dragged in one undo step
  (`app/integration_test/handles_test.dart`).
- Open (step 4, core work): constraints on internal lines, notches and
  construction lines; holes in mirror copies and symmetric pieces; seam
  allowance inside holes; moving a dimension's label by hand.

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
- Delivered, first half (2026-11-18, spec 23): fabric details on each
  garment (weight g/m², width, price per metre; schema v14; the weight
  is also the drape's mass), indicators per piece and per garment
  (`production.indicators`: area, cut area with the allowance,
  perimeter, mass), the marker (`marker.compute`: one garment in one or
  several sizes on a fabric width, cutting lines with the allowance,
  grain along the length at 0°/180° or free at 45° steps, quantities and
  mirrored pairs; raster bottom-left-fill after Jakobs 1996 and Burke et
  al. 2006 in `r3d_geom::nest`; length, efficiency, yardage, cost), its
  export at 1:1 (`export.marker`, `r3d marker`, SVG and PDF), and the
  app (fabric fields and totals under each garment, cut area and mass
  of a piece, Export → Marker… dialog with sizes, width, the layout to
  scale and its figures, Save SVG/PDF). A shirt in sizes 38, 40 and 42
  on 150 cm: 2.80 m, 78 % of the fabric used.
- Delivered, second half (2026-11-19, spec 23): a garment's notes
  (`garment.set_notes`, schema v15; a Notes field under the garment in
  the app); the tech pack (`export.techpack`, `r3d techpack`, the MCP
  tool `export_techpack` with `views` 3d/photo/none, Export → Tech
  pack… in the app): a landscape PDF laid out by `r3d_io::techpack`
  with the garment's front and back, colourways, the bill of materials
  (each garment's fabric, the fabric per garment from its marker, in
  every size when graded; trims empty until M20), the measurements
  (size chart, pieces in the sample size, pieces by size), the pattern
  to scale at an exact 1:N with the sizes' legend, and the sewing notes
  (seams, notes). Its words are labels with stable keys and English
  defaults; the app passes its own (French or English). The projector
  (Export → Projector… in the app): the selected pieces in the shown
  size, at 1:1 once calibrated by measuring a 20 cm square on the
  table, white on black or black on white, mirrored, turned, panned
  with the mouse or the arrows, a 10 cm scale bar; the calibration is
  an app preference (`projector.json`), not in the project.
- **Demo**: `./demos/m30/demo.sh` (the graded shirt's marker, then its
  tech pack; the projector is a step in the app, described there and in
  the user guide).
- **Open** (later): trims in the bill of materials (M20); the projector
  in the operating system's own full screen (the view fills the app's
  window today) and keystone correction; points of measure named by
  the user; true-shape nesting (no-fit polygons) and a gap between
  pieces, one-way fabrics (nap, prints), folded fabric and several
  plies, a marker of several garments in one fabric, PDF pages longer
  than 5 m for Acrobat.

## M31 — Imported avatars ✅ (2026-11-19)
- Import a body in OBJ, glTF or GLB (a scan, a game character); the
  user owns its licence.
- A check of the collision mesh (closed, scale, orientation, holes)
  shown before draping; the arrangement points placed with help and
  checked before the simulation starts.
- **Demo**: `./demos/m31/demo.sh`: the built-in CC0 body exported to
  OBJ, rewritten in centimetres, Z up, its soles cut open, is imported
  (units, up axis and facing detected; the check reports the holes;
  14 arrangement points guessed inside the body; measurements within
  half a centimetre of the built-in body's), turned the wrong way and
  flagged, corrected, a T-shirt sized from it draped and rendered.
- Delivered (spec 27): schema v18, the avatar's `mesh` (file in
  `avatars/` by relative path and fingerprint, scale, up axis, turn,
  arrangement points); `r3d_io::model` (OBJ, glTF/GLB with embedded
  buffers and node transforms); `r3d_geom::trimesh` (holes,
  non-manifold edges, pieces, outward orientation fixed) and
  `r3d_geom::winding` (generalized winding numbers, Jacobson et al.
  2013, Barill et al. 2018's hierarchy) signing the distance field near
  open or non-manifold edges, on the CPU and the GPU;
  `r3d_commands::bodies` (files kept by fingerprint, detection of units
  from a plausible height, up axis, facing from the shoulders and the
  feet; points from the body's sections; measurements as the built-in
  body's; the check report; the dress form with sleeves and trouser
  legs through the points). Commands `avatar.import_mesh`,
  `avatar.orient`, `avatar.set_point`, `avatar.guess_points`,
  `avatar.remove_mesh`, query `avatar.check`, action `avatar.import`;
  `r3d avatar import|check`; `simulate` refuses motions on an imported
  body and reports `body_warnings`. App: Body menu (Measurements…,
  Import body…, Check body…, Show arrangement points, Use the built-in
  body), a dot on it when something needs a look, the check window
  (units, up axis, turn, the items, the points in cm), the points drawn
  over the 3D view and dragged, the Motion button off, a notice when
  the drape starts; integration test `body_test.dart`.
- Open: motions of an imported body (mapping its skeleton, if any,
  onto ours), an imported body per size of a range, automatic fit of
  the arrangement points as the garment drapes, points guessed on a
  T pose (they assume arms down or an A pose), templates placing their
  pieces from the points rather than from fractions of the height, a
  disk cache of the distance field of large scans, compressed glTF.

## M23 — Animated avatars
- A skeleton for the avatar and a few motions (walk, turn, raise arms);
  the cloth follows the moving body. Animation assets under a
  redistributable licence only (CLAUDE.md §1).
- Delivered (spec 22): MakeHuman's default skeleton and weights (CC0)
  reduced to 24 bones in the body pack, fitted with every body; our own
  procedural motions (walk on the spot, half a turn and back, arms raised
  above the shoulders), eased in and out, the body back at rest before
  the next one; the cloth collides with the moving skin through inverse
  linear blend skinning of the rest pose's distance field, with friction
  relative to the skin and pre-stabilised contacts, on the CPU and the
  GPU (physics.md §5, Moving body); the app's Motion menu; `simulate`
  with `motion` for agents and the CLI, cached with the drape, so renders
  and exports show the posed body.
- **Demo**: `./demos/m23/demo.sh` (a dress walking, a skirt turning, a
  T-shirt raising its arms, rendered and exported at the last frame).
- Left for later: walking along a path, arms straight overhead (needs a
  skinning that does not fold the armpit, e.g. dual quaternions),
  imported avatars' motions (M31), a timeline and a video of a motion.
- Captured motions (2026-11-19, spec 22): a BVH motion capture imported
  into the project plays on the avatar, retargeted to its skeleton and
  looped on the spot; the Motion menu's import, `simulate`'s
  `motion_file`. First tried with a slow walk of the CMU database
  (maintainer: a good start). A motion library shared by all projects:
  add to it after an import, manage it in a dialog (add files, play,
  rename, delete), its motions in the Motion menu.
- A more natural walk and a standing motion (2026-11-19): the walk's
  thigh no longer forward on average (22° forward to 16° back), the
  knee bending before toe-off, the pelvis dropping on the swing side,
  level shoulders and a steady head, easier arms (ranges checked
  against the CMU slow walk, kept smaller); `idle` (Stand at ease): the
  weight shifting from leg to leg, breathing, over 8 s. The relaxed arms
  hang 10° closer to the body.
- Fixed (2026-11-19; maintainer, seen with a captured walk): the hands
  passed through the hips and caught the garment, which got pulled, and
  trousers slid down and up again as the avatar walked. One cause: the
  capture's arms hang closer than the avatar's hips allow. Each pose now
  turns the arms out just enough for the forearms and hands to clear
  the body by 15 mm (physics.md §5, Moving body); the trousers stay
  within about 1.5 cm instead of sliding 7–10 cm.

## M24 — Fit maps and virtual tape measure
- Stress and strain shown in colour on the draped garment, to find
  where it is tight or loose.
- A tape measure on the drape: circumferences of the garment and ease
  (the gap between body and garment) read where the user clicks.
- **Demo**: `./demos/m24/demo.sh`: a draped T-shirt rendered with its
  strain and gap maps to PNG; strain per piece, the tight spots, and
  the ease at the chest, waist and hips.
- Delivered (2026-11-19, spec 24): `r3d_commands::fit`, computed from
  positions only, so the app's live drape (GPU or CPU) and the cached
  headless one read alike. Strain per triangle (largest principal
  stretch of the deformation gradient against the pattern, Bonet &
  Wood 1997), averaged at the vertices; gap per vertex to the body
  (posed if it moves). One colour scale for both, viridis (dark loose,
  yellow tight; colour-blind safe), fixed ranges (strain 0–20 %, gap
  50–0 mm), drawn as per-vertex colours by the 3D view
  (`r3d_render::fitmap`, `Mesh::colors`). The tape: a horizontal cut of
  the cloth joined across seams, its length along the cloth; the body's
  cut inside it by tape (convex hull); ease = garment − body; the gap
  at a clicked point. Agents: `fit.report` (per piece max/mean strain,
  share tight, mean gap, share touching; the tight spots with piece,
  peak, area, height and pattern position; ease at chest, waist and
  hips and at heights asked), `fit.tape` (at a height or a point),
  `render_3d` / `r3d render --view 3d --fit-map strain|gap` (a scale bar
  at the right edge), `r3d fit`. App: the gauge button over the 3D
  view (Fabric colours, Strain, Gap to the body) with a legend and the
  tightest value; the Tape measure tool (click the drape: the loop drawn
  on the cloth, a card with height, garment, body, ease, gap). The
  T-shirt on the women's M body: 11.7 cm of ease at the chest, sleeves
  up to 20–40 % strain under the arms.
- Delivered (2026-11-20): the tape round a point is no longer always
  level: it goes square to the way the cloth runs there (a slanting
  sleeve, an arm raised), level round the torso; a length between two
  points of the drape, along the cloth, by tape (taut over folds) and
  straight (the Tape measure card's Length mode, `fit.length`).
- Open: a stress map (force per width: strain times the fabric's
  stiffness per direction) and a pressure map; the tape following the
  cloth as it moves (it is taken at the click); the length of an
  opening's edge (a neckline, an armhole).

## M15 — Game-ready export ✅ (2026-11-19)
- Decimated LODs, wrinkle detail baked into normal maps, back faces or
  thickness; skinning to the avatar skeleton (M23); presets for Godot,
  Unity and Unreal.
- **Demo**: `./demos/m15/demo.sh`: a striped T-shirt draped on the
  women's M body, exported with three LODs (3274, 1636, 654 triangles),
  normal maps baked from the full drape, the stripes in an atlas, skinned
  to the 24-bone skeleton with walk, turn and raise_arms as animations
  and the body; then for Unreal as a 1.5 mm shell; both files pass the
  Khronos glTF Validator with no error or warning.
- Delivered (2026-11-19, spec 15): action `export.game` (MCP
  `export_game`), `r3d export --format game` (`--preset`, `--lods`,
  `--normal-map`, `--shell`, `--thickness-mm`, `--no-skin`,
  `--no-animations`); `r3d_geom::decimate` (quadric error metrics,
  Garland & Heckbert 1997, half-edge collapses keeping borders and
  refusing flips in 3D and in the atlas), `r3d_geom::bake` (tangents,
  normal maps through the shared atlas after Cohen, Olano & Manocha
  1998, colour baking, padding); `r3d_io::game` (skins, animations,
  tangents, normal and colour maps); `r3d_commands::game` (one mesh per
  garment and LOD, the atlas, closest-point skin weight transfer
  smoothed over the garment, a drape that ended mid-motion carried back
  to the rest pose, the motions baked at 30 fps, presets and names).
  `scripts/validate-exports.sh` validates two game exports.
  App: Export › Garment for a game engine (glTF)… opens a dialog
  (engine, levels of detail in %, normal map size, double-sided or
  thickness, skin to the skeleton, include the body; the engine fills
  the fields; skinning off and explained for an imported body);
  `Engine::export_game` exports the drape the 3D view shows. Strings in
  en and fr, error codes translated; integration test `game_test.dart`.
- Open: imports checked by hand in Godot, Unity and Unreal (the presets
  follow their documented glTF importers; no engine was run); the
  `MSFT_lod` extension (LODs are separate `_LODn` nodes); skinning an
  imported body (no skeleton, spec 27); dual-quaternion skinning; the
  inside of a shell shows the outside's wrinkles reversed; roughness or
  sheen maps.

## Later (architecture kept ready, not scheduled)
Material calibration from measurements (a home protocol: a strip
hanging over an edge for bending, a weight and a ruler for stretch; a
community fabric library under CC0; only data whose licence allows
commercial use) · turntable video of the photo render (a runway walk
after M23) · step-by-step sewing instructions generated from the seams
(needs a sewing order) · OpenUSD
export · finer cloth mesh with a strain limit (smaller folds;
physics.md §7) · picking points and notches in 3D · signed packages.
