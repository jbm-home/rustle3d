# 10 — CLI

Provenance: maintainer's functional brief (2026-11-14).

## User story
As a developer, tester or agent, I drive Rustle3D without a window: run
scripts, simulate, export and render.

## Behavior
`r3d` binary (clap). Implemented: `new`, `run`, `query`, `schema` (M1),
`export`, `mcp` (M2), `simulate`, `render` (M4), `import` (M13). `run
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
  [--page n]`: copy a DXF-AAMA (or plain DXF), SVG or PDF pattern into
  the project's `imports/` folder, import its pieces (the
  `import.pattern` action, one undo step) and save (see 07).
- `r3d render <project> --view board|3d --out file.png [--size WxH]`: the
  3D view shows the cached drape when it matches the document, otherwise
  the starting positions; the JSON output says which (`"drape"`).
- `r3d mcp --project path`: MCP server over a folder; `r3d mcp --attach
  [--endpoint file]`: relay to the project open in the app (see 09).
- `r3d export <project> --format svg|pdf|dxf|glb|obj [--paper a4|letter|single]
  [--avatar] [--out path]`: the pattern, or the draped garment (see 07, 15).
- `r3d schema [command]`: print JSON schemas.
- Output is machine-readable JSON by default when piped. Exit codes: 0 ok,
  1 user error, 2 internal error.

## Acceptance criteria
- Each subcommand has an integration test using `assert_cmd`.
- Every milestone demo script runs with the CLI only.
