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
| M9 | glTF export, attached MCP | 2 wk | An agent edits the garment live in the open app; the result opens in a game engine |
| M10 | Templates, guided flow, polish → **v0.1** | 3–4 wk | A first-time user makes a T-shirt in < 10 minutes |
| M11 | Photorealistic rendering | 4–6 wk | A path-traced studio image of the draped garment |
| M12 | Game-ready export | 3–4 wk | LODs + baked wrinkle normal maps, imported in Godot/Unity/Unreal |

Total: roughly 7.5–8.5 months to v0.1, then M11–M12.

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

## M9 — glTF export, attached MCP
- glTF 2.0/GLB garment export with pattern UVs and `KHR_materials_sheen`;
  OBJ; glTF validator in CI; checked imports in Blender and Godot.
- Attached mode: app-hosted MCP over a local socket, `r3d mcp --attach`,
  agent indicator, disconnect button.
- **Demo**: the agent changes the skirt length while the user watches it
  re-drape live.

## M10 — Templates, guided flow, polish → v0.1 (in progress)
- Parametric templates (skirts, T-shirt, dress, tote bag), guided flow step
  bar, Learn panel, first-launch tour.
- Usability pass and the 10-minute test with first-time users.
- Autosave/recovery, release builds for macOS, Windows and Linux.
- **Demo**: a first-time user makes a T-shirt in under 10 minutes.

Done so far (2026-11-16), demo `./demos/m10/demo.sh`:
- `template.apply` / `template.list`: T-shirt, straight skirt and A-line
  skirt drafted from sizes (the avatar's by default), sewn, placed and
  given a fabric through ordinary commands; each drapes in a test.
- Sleeves: schema v6 placements wrap a panel around the body or an arm
  (the dress form finds its upper arms). Gravity ramps in while seams
  close. The dress form's bust is smoothed; skirts hang from the measured
  waist height.
- App: start screen (template cards with rendered thumbnails, sizes sheet
  per body, blank project, recent projects in Documents/Rustle3D, saved as
  you work), back to projects, live step bar Measurements → Pattern → Sew →
  Fabric → Drape with a goal, an action and a highlighted tool per step.
- From the M8 leftovers: style lines and the selected edge drawn on the 3D
  garment (also in agents' renders); seam direction chevrons and red
  mismatched seams on the board; camera views (front, side, back, ¾; keys
  1–4).

Still to do: dress and tote bag templates, Learn panel and first-launch
tour, the 10-minute test with first-time users, release builds; picking
points and notches in 3D, per-panel placement and dragging panels in 3D.
Known: XPBD creep lets a skirt slide a few cm below the waist before it
settles (physics.md §7 convergence); a speck of the body can show at the
bust through a tight top.

---

## M11 — Photorealistic rendering
- Progressive GPU path tracer in `r3d-render` (wgpu compute, hardware ray
  queries when available), physically based fabric materials (sheen, fuzz,
  thickness), HDRI studio lighting, denoising; renders in the background and
  refines in the 3D view; `render_photo` for CLI/MCP.

## M12 — Game-ready export
- Decimated LODs, wrinkle detail baked into normal maps, back faces or
  thickness; skinning to the avatar skeleton once avatars are animated;
  presets for Godot, Unity and Unreal.

## Later (architecture kept ready, not scheduled)
Grading and sizes · DXF-AAMA/ASTM import/export · animated avatars · layers
and multiple garments · trims (buttons, zippers) · textures and prints with
UVs from the pattern · fit maps (stress/strain) · material calibration from
measurements · GPU solver backend / Vertex Block Descent · OpenUSD export ·
French UI translation.
