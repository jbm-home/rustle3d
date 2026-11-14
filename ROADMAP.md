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
| M1 | Document + commands + CLI ✅ (in review) | 1.5–2 wk | CLI script builds panels, undo/redo, save/reopen |
| M2 | Geometry, 2D export, headless MCP | 2 wk | An agent drafts a skirt pattern via MCP and exports a tiled PDF |
| M3 | GUI spike, desktop shell, design system, 2D editor | 4–5 wk | Draw and edit panels in a polished UI (**visual gate**) |
| M4 | Simulation v1 | 3 wk | A pinned panel drapes in real time in the 3D view |
| M5 | Avatar, collisions, arrangement | 3 wk | Panels arranged around the avatar fall onto it |
| M6 | Sewing + synced views | 3 wk | A skirt made from scratch, sewn and draped |
| M7 | Materials, self-collision, grab/pin | 2–3 wk | The same T-shirt in jersey vs denim; pull it with the mouse |
| M8 | 3D export, attached MCP | 2 wk | An agent edits the garment live in the open app |
| M9 | Templates, guided flow, polish → **v0.1** | 3–4 wk | A first-time user makes a T-shirt in < 10 minutes |

Total: roughly 6.5–7.5 months to v0.1.

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
- `r3d-geom`: cubic Bézier (length, split, nearest point), symmetry, offset
  for seam allowance, constrained triangulation at a given spacing (prototype
  `kurbo` vs our own).
- Commands: curves, mirror (linked), notches, internal lines, grainline,
  measure queries.
- `r3d-io`: SVG and tiled PDF export with the 100 mm test square.
- `r3d-mcp`: stdio server generated from the registry (read and write tools,
  batch, undo/redo, path confinement).
- **Demo**: an agent session connected to `r3d mcp` drafts a skirt pattern and
  exports `exports/skirt.pdf`.

## M3 — GUI stack spike, desktop shell, design system, 2D editor
- **Spike (≈1 week, first)**: the same small screen in the 2–3 strongest GUI
  candidates (architecture §7.1). Pick the stack with the maintainer and
  record it in the decision log.
- Design system in the chosen stack: tokens, light/dark themes, fonts, icon
  set, core widgets, motion; i18n message catalog (`en`).
- `r3d-render`: offscreen board renderer → MCP `render_board`.
- Editor: selection tool, rectangle/polygon/Bézier tools, snapping, inline
  dimensions with `Tab`, undo history panel, command palette, status bar hints.
- **Visual gate**: screenshots of the start screen, editor and inspector in
  both themes, reviewed before going further.
- **Demo**: draw a skirt by hand; an agent renders the board to PNG.

## M4 — Simulation v1
- `r3d-sim`: `Solver` trait, XPBD small steps, strain-based stretch/shear,
  quadratic bending, pins, gravity, ground plane, graph coloring + rayon,
  deterministic mode, `criterion` benchmarks.
- Sim bridge (core → `SimScene`), simulation thread, triple buffer.
- 3D view: camera presets, cloth shading; `render_3d`.
- CLI `simulate`; reference scenes 1 and 5.
- **Demo**: `r3d simulate demo.r3d --steps 300 && r3d render demo.r3d --view 3d --out drape.png`.

## M5 — Avatar, collisions, arrangement
- Vendored MakeHuman CC0 avatars (decimated, glTF), provenance recorded.
- OBJ/glTF avatar import, SDF builder (winding numbers) + cache, friction.
- Arrangement points, wrap placement, "Arrange all".
- Reference scene 2.
- **Demo**: two rectangles arranged front/back fall onto the avatar.

## M6 — Sewing + synced views
- Seam model, commands and validation; seam tool with mismatch badges and
  direction arrows; seam resampling; sewing phase and welding.
- Shared selection/hover between 2D and 3D, picking in 3D.
- Reference scenes 3 and 4.
- **Demo**: skirt from scratch → sewn → draped, all reproducible by a CLI script.

## M7 — Materials, self-collision, grab/pin
- Fabric presets and material library; simple/advanced material inspector.
- Particle self-collision with spatial hashing.
- Mouse grab and pins in 3D; live remeshing after edits while simulating.
- Hit the performance targets in `docs/physics.md` §7.
- **Demo**: T-shirt in jersey vs denim, pulled with the mouse.

## M8 — 3D export, attached MCP
- OBJ/glTF garment export with UVs; glTF validator in CI.
- Attached mode: app-hosted MCP over a local socket, `r3d mcp --attach`,
  agent indicator, disconnect button.
- **Demo**: the agent changes the skirt length while the user watches it
  re-drape live.

## M9 — Templates, guided flow, polish → v0.1
- Parametric templates (skirts, T-shirt, dress, tote bag), guided flow step
  bar, Learn panel, first-launch tour.
- Usability pass and the 10-minute test with first-time users.
- Autosave/recovery, release builds for macOS, Windows and Linux.
- **Demo**: a first-time user makes a T-shirt in under 10 minutes.

---

## After v0.1 (architecture kept ready, not scheduled)
Grading and sizes · DXF-AAMA/ASTM import/export · animated avatars · layers
and multiple garments · trims (buttons, zippers) · textures and prints with
UVs from the pattern · fit maps (stress/strain) · material calibration from
measurements · GPU solver backend / Vertex Block Descent · offline
high-quality mode · rendering · French UI translation.
