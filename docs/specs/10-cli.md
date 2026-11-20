# 10 — CLI

Provenance: maintainer's functional brief (2026-11-14).

## User story
As a developer, tester or agent, I drive Rustle3D without a window: run
scripts, simulate, export and render.

## Behavior
`r3d` binary (clap). Implemented: `new`, `run`, `query`, `schema` (M1),
`export`, `mcp` (M2), `simulate`, `render` (M4), `import` (M13),
`marker`, `techpack` (M30), `fit` (M24), `texture` (M21), `avatar`
(M31). `run
--dry-run` uses a bare session: actions such as exports are refused and
nothing is written. Scripts can refer to earlier results
(`"$3.created[0]"`), see `docs/commands.md`.
- `r3d new <path>`: create an empty project folder (optionally `--template skirt`).
- `r3d run <project> <script.json|->`: execute a JSON list of commands (same
  format as the MCP batch) and save. Use `--dry-run` to validate only.
- `r3d query <project> <query-json>`: print query results as JSON.
- `r3d simulate <project> [--frames N]`: drape the placed, sewn garment on
  the avatar for N frames of 1/60 s (default 300) and print a report
  (extent, seam gap, settled). Always deterministic. The result is cached
  in `.r3d-cache/drape.bin`, keyed by a fingerprint of the document, so a
  stale drape is never shown. Same as the `simulate` action (scripts, MCP).
- `r3d import <project> <file> [--units mm|cm|m|in|pt|px] [--scale k]
  [--page n] [--measurements file]`: copy a DXF-AAMA (or plain DXF), SVG,
  PDF, Seamly2D (`.sm2d`) or Valentina (`.val`) pattern into the
  project's `imports/` folder (a parametric pattern with the measurement
  file it names, or `--measurements`), import its pieces (the
  `import.pattern` action, one undo step) and save (see 07 and 28).
- `r3d measurements <project> <file>`: copy a Seamly2D or Valentina
  measurement file (`.vit`, `.smis`, `.vst`, `.smms`) into `imports/`,
  take the avatar's body, variables and (multisize) sizes from it
  (`import.measurements`) and save (see 28).
- `r3d render <project> --view board|3d|photo --out file.png [--size WxH]`:
  the 3D view and the photo show the cached drape when it matches the
  document, otherwise the starting positions; the JSON output says which
  (`"drape"`). A photo (M14, spec 14) takes `--quality quick|good|best`,
  `--lighting studio|soft|dramatic`, `--camera
  front|three-quarter|side|back`, `--frame garment|body`, `--samples N`,
  `--bounces N` and `--no-denoise`; its output adds the samples and the
  seconds taken. `--fit-map strain|gap` (3D only, M24, spec 24) colours
  the cloth by how tight it is, with a scale bar at the right edge; the
  output adds the scale and the tightest value.
- `r3d texture <project> <image|stripes|gingham|dots|twill> [--garment
  id] [--width-mm W] [--height-mm H] [--angle DEG]`: put a print on a
  garment (spec 25): a PNG or JPEG is checked and copied into the
  project's `textures/` folder, a built-in is drawn in the garment's
  colour; runs `garment.set_print`, saves, and prints the garments.
- `r3d avatar import <project> <file> [--scale k] [--up y|z|x|-y|-z|-x]
  [--turn DEG]`: copy an OBJ, glTF or GLB body into the project's
  `avatars/` folder, drape on it (the `avatar.import` action, one undo
  step; units, up axis and facing detected unless given), save, and
  print the measurements taken on it with its check (spec 27).
  `r3d avatar check <project>`: the check alone (`avatar.check`).
- `r3d fit <project> [--threshold-pct P] [--heights-cm 100,120]`: how the
  cached drape fits (`fit.report`, spec 24): strain per piece, the tight
  spots, the ease at the chest, waist and hips (and at the heights
  given).
- `r3d mcp --project path`: MCP server over a folder; `r3d mcp --attach
  [--endpoint file]`: relay to the project open in the app (see 09).
- `r3d export <project> --format svg|pdf|dxf|glb|obj|game [--paper a4|letter|single]
  [--avatar] [--sizes 36,44|all] [--out path]`: the pattern (in some sizes
  of its range, nested, with `--sizes`; spec 18), or the draped garment
  (see 07, 15). `--format game` (M15, `export.game`) takes `--preset
  generic|godot|unity|unreal`, `--lods 1,0.5,0.2`, `--normal-map N`,
  `--shell [--thickness-mm T]`, `--no-skin`, `--no-animations`.
- `r3d marker <project> [--sizes 38,40|all] [--width-cm W] [--garment id]
  [--format svg|pdf] [--paper single|a4|letter] [--out path]`: lay the
  garment's pieces out on the fabric and print the length, efficiency,
  yardage, cost and where each piece lies (`marker.compute`); with
  `--format`, also write the marker at 1:1 (`export.marker`; spec 23).
- `r3d techpack <project> [--views 3d|photo|none] [--paper a4|letter]
  [--out path]`: write the tech pack PDF (`export.techpack`, spec 23)
  into the project folder, opening with the garment's front and back
  rendered from the cached drape (`--views none`: no pictures, no GPU
  needed); prints the path and the fabric per garment.
- `r3d schema [command]`: print JSON schemas.
- Output is machine-readable JSON by default when piped. Exit codes: 0 ok,
  1 user error, 2 internal error.
- Long steps (M26: a template drafted in every size, a drape) print their
  progress on one rewritten stderr line (`drafting sizes: 3/9 (40)`),
  only when stderr is a terminal: stdout stays the JSON result.

## Acceptance criteria
- Each subcommand has an integration test using `assert_cmd`.
- Every milestone demo script runs with the CLI only.
