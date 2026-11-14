# 10 — CLI

Provenance: maintainer's functional brief (2026-11-14).

## User story
As a developer, tester or agent, I drive Rustle3D without a window: run
scripts, simulate, export and render.

## Behavior
`r3d` binary (clap):
- `r3d new <path>`: create an empty project folder (optionally `--template skirt`).
- `r3d run <project> <script.json|->`: execute a JSON list of commands (same
  format as the MCP batch) and save. Use `--dry-run` to validate only.
- `r3d query <project> <query-json>`: print query results as JSON.
- `r3d simulate <project> --steps N [--deterministic] [--save-drape]`.
- `r3d export <project> --format svg|pdf|obj|gltf [--out path]`.
- `r3d render <project> --view board|3d --out file.png [--size WxH]`.
- `r3d mcp [--project path] [--attach]`: MCP server (see 09).
- `r3d schema [command]`: print JSON schemas.
- Output is machine-readable JSON by default when piped. Exit codes: 0 ok,
  1 user error, 2 internal error.

## Acceptance criteria
- Each subcommand has an integration test using `assert_cmd`.
- Every milestone demo script runs with the CLI only.
