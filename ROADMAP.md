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
| M8 | 3D → 2D editing tools | 3 wk | Lengthen a hem and take in a seam directly in 3D; the pattern follows |
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
- One template source (`templates/a-line-skirt.json`).

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

## M8 — 3D → 2D editing tools

Status (2026-11-15): live re-simulation is in: pattern, fabric and pin
edits reach the running drape within 100 ms, positions carried over
(`drape::carry_over`), no restart. The Length tool is in: ⇧-drag a hem in 3D (`edge.lengthen`, the
whole hemline moves along the grain). Fit is in too: ⇧-drag a seam
(`seam.take_in`, tapered when grabbed near an end). Next: the M8 demo
(shorten and take in the waist from a script), then Pick and Draw.
- From M4–M7: live re-simulation after pattern edits (positions carried
  over, so editing in 2D updates the drape without a restart); sewing and
  arranging in the UI (seam tool with mismatch badges and direction
  arrows, placement around the body, "Arrange all"); camera presets.
- Pattern coordinates on every simulated vertex; ID-buffer picking.
- Tools: lengthen/shorten by dragging an edge in 3D, pinch to take in or let
  out a seam, draw style lines on the garment, select pattern elements in
  3D. All of them emit ordinary pattern commands (architecture §7.8).
- **Demo**: shorten a skirt and take in its waist in 3D; the pattern and its
  PDF export follow.

## M9 — glTF export, attached MCP
- glTF 2.0/GLB garment export with pattern UVs and `KHR_materials_sheen`;
  OBJ; glTF validator in CI; checked imports in Blender and Godot.
- Attached mode: app-hosted MCP over a local socket, `r3d mcp --attach`,
  agent indicator, disconnect button.
- **Demo**: the agent changes the skirt length while the user watches it
  re-drape live.

## M10 — Templates, guided flow, polish → v0.1
- Parametric templates (skirts, T-shirt, dress, tote bag), guided flow step
  bar, Learn panel, first-launch tour.
- Usability pass and the 10-minute test with first-time users.
- Autosave/recovery, release builds for macOS, Windows and Linux.
- **Demo**: a first-time user makes a T-shirt in under 10 minutes.

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
