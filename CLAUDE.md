# CLAUDE.md — rules for AI agents working on Rustle3D

Rustle3D is an open-source (GPL-3.0-or-later) desktop application in Rust
for 3D garment design: draft sewing patterns in 2D, sew them, drape them on
an avatar with real-time cloth physics. It must stay simple to use and fully drivable by AI
agents through its command layer (CLI and MCP).

Read `docs/architecture.md` before touching code, `docs/physics.md` before
touching `r3d-sim` or `r3d-sim-gpu`, `docs/rendering.md` before touching
`r3d-render`'s photo renderer, and the relevant `docs/specs/*.md` before
implementing a feature. `ROADMAP.md` says which milestone we are on.

**Firm vs. flexible.** Only the clean-room rules (section 1) are fixed. They
protect the project legally. Everything else (engineering conventions,
crate split, libraries, solver, UI stack) is the current best practice for
this stage of development. If a rule gets in the way or a better approach
appears, say so with arguments and propose a change. Do not silently work
around it. Record accepted changes in the change log of
`docs/architecture.md`.

## 1. Clean-room rules (non-negotiable)

This is an independent, clean-room implementation.

- **Never** install, run, drive, screenshot, decompile or inspect any
  proprietary garment software (CLO3D, Marvelous Designer, Browzwear, Style3D,
  Optitex, …), nor its file formats, binaries, assets or material libraries.
  Do not search for, fetch or summarize their documentation or UI screenshots
  either.
- Specs come only from: the maintainer's functional descriptions, public
  academic papers, open standards (DXF-AAMA/ASTM, glTF, OBJ, SVG, PDF) and
  general garment-making knowledge. Every spec in `docs/specs/` has a
  `Provenance:` line; keep it accurate when you edit a spec.
- No copied UI: our own layout, icons, colors, terminology and names. No
  third-party trademarks in product names, UI strings or code identifiers.
- No proprietary file format support based on reverse engineering. Interop
  goes through open formats only.
- Avatars: only assets whose license allows commercial redistribution (e.g.
  MakeHuman CC0). **Never** SMPL, SMPL-X or any non-commercial body model.
  Record provenance in `assets/avatars/README.md`.
- Other open-source projects may inspire design ideas. Code may be copied
  or closely adapted only from projects whose license is compatible with
  GPL-3.0-or-later (MIT, Apache-2.0, BSD, Zlib, MPL-2.0, LGPL/GPL v3, …;
  never GPL-2.0-only); keep the original copyright notice and add an entry
  to `THIRD_PARTY_NOTICES.md`. Prefer reimplementing from papers.
- Physics and rendering algorithms come from published papers; cite them in
  the doc comment of the implementing code and in `docs/physics.md` or
  `docs/rendering.md`.

If a request would break one of these rules, stop and say so.

## 2. Engineering conventions (current, revisable)

- Rust stable (see `rust-toolchain.toml`), edition 2024.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` must pass.
- **Never-crash policy**: no `unwrap()`, `expect()`, `panic!`, `todo!`,
  `unimplemented!` or unchecked indexing that can fail on user data in
  non-test code. These lints are denied in `Cargo.toml` (integration test
  files opt out with a crate-level `#![allow]`). Use error types built with
  `thiserror` per crate; the app reports errors, it never aborts.
- **Clean code**: no dead code, no unused API "for later", no workaround
  stacked on a previous fix. When a fix shows the design was wrong, fix the
  design. If an approach keeps failing, step back and change approach
  instead of insisting.
- **i18n**: no user-facing prose in core crates. Errors implement
  `r3d_core::Localizable` (stable `code` + `args` + English fallback).
- No `unsafe` (denied workspace-wide). Discuss before lifting it for one module.
- **All business logic goes through `r3d-commands`.** The UI, CLI and MCP
  never mutate the document directly; they build commands. If you need a new
  operation, add a command (with validation, schema, undo test) first.
- Units: the document stores 2D in millimetres (`f64`); the simulation works
  in metres (`f32`), Y-up, right-handed (glTF convention). Convert only at
  the core→sim boundary.
- IDs are stable typed newtypes allocated by the document; never use
  indices into vectors as identity.
- New dependencies: licenses compatible with GPL-3.0-or-later. Prefer
  permissive ones (MIT, Apache-2.0, BSD, Zlib, ISC, CC0, Unicode); MPL-2.0,
  LGPL-3.0 and GPL-3.0 are acceptable with a justification. Never
  GPL-2.0-only, non-commercial or proprietary. Justify each new dependency in the PR.
- Keep the crates' dependency direction as documented in
  `docs/architecture.md`. `r3d-core`, `r3d-geom` and `r3d-sim` never depend
  on GUI or GPU crates.

### Tests

- Unit tests for geometry and commands; property tests (`proptest`) for
  geometry invariants; every command has an apply → undo → redo round-trip
  test.
- Golden-file tests for serialization (`tests/golden/`); a schema change
  needs a migration and a new golden file, never an edited old one.
- Simulation regression tests run in deterministic mode on reference scenes
  and check: no NaN/inf, bounded energy, no particle inside the avatar SDF
  beyond tolerance, seams closed within tolerance.
- Benchmarks for the solver with `criterion` (`cargo bench -p r3d-sim`).

## 3. Commands

```sh
./scripts/check.sh                           # run before EVERY commit; commit only if it exits 0 (never through a pipe)
./scripts/app-screenshot.sh out.png          # capture the app window for visual review
cargo build                                  # whole workspace
cargo test                                   # all tests
cargo test -p r3d-core                       # one crate
cargo clippy --all-targets -- -D warnings    # lints, as in CI
cargo fmt --all                              # format
cargo run -p r3d-cli -- --help               # headless CLI (binary: r3d)
cargo xtask gen-docs                         # regenerate docs/commands.md (a test checks it)
UPDATE_GOLDEN=1 cargo test -p r3d-core       # regenerate current-version golden files
./demos/m30/demo.sh                          # latest milestone demo (one folder per milestone)
cargo bench -p r3d-sim                       # CPU solver benchmarks (docs/physics.md §7)
cargo bench -p r3d-sim-gpu                   # GPU vs CPU on the T-shirt and the dress
cargo run -p r3d-cli -- simulate X.r3d       # headless drape (cached), then: render X.r3d --view 3d|photo --out a.png
cargo run -p r3d-cli -- mcp --project X.r3d  # MCP server over stdio for agents
(cd app && flutter run -d macos)             # desktop app (Flutter UI + Rust core)
(cd app && flutter_rust_bridge_codegen generate)  # after changing crates/r3d-ffi/src/api
(cd app && flutter test integration_test -d macos)  # UI <-> Rust integration tests
```

If `cargo` is not on `PATH`, run `source ~/.cargo/env` first.

## 4. Crate map

| Crate          | Role                                                                 |
|----------------|----------------------------------------------------------------------|
| `r3d-geom`     | 2D math: Bézier, polylines, offsets, polygon ops, panel triangulation |
| `r3d-core`     | Document model, typed IDs, versioned JSON + migrations               |
| `r3d-sim`      | XPBD cloth solver (CPU/rayon) behind a `Solver` trait, collisions    |
| `r3d-sim-gpu`  | The same solver on wgpu compute (the app's backend)                  |
| `r3d-io`       | OBJ, glTF, SVG, PDF, DXF-AAMA export; DXF, SVG, PDF pattern import    |
| `r3d-commands` | Typed commands, validation, JSON schemas, undo/redo, `Session`       |
| `r3d-render`   | wgpu renderers: 2D board, 3D view, path-traced photos; on-screen and offscreen |
| `r3d-mcp`      | MCP server over the command layer (stdio or local socket)            |
| `r3d-cli`      | `r3d` binary: scripts, simulate, export, render, `mcp`               |
| `r3d-ffi`      | Bridge for the Flutter desktop app (`app/`): engine, views, libraries |

## 5. Workflow

- Small, reviewable commits; one feature per branch/PR.
- At the end of each milestone: run all tests, update docs (architecture,
  physics, specs, regenerate `docs/commands.md`), and provide a short demo
  script (CLI or MCP) that exercises the new feature.
- Documentation and code are in English.
- **User guide for the website**: `docs/user-guide.md` is the source of
  the user documentation on rustle3d.com (plain words, for users, not
  developers). Every user-visible change (tool, shortcut, menu, format,
  limit) updates it in the same commit, and its "Last updated" line.
- **End of a piece of work**: tell the user whether to go on in this
  session or start a new one (a new one when the context is long or the
  next task is unrelated). Before suggesting a new session, make sure
  nothing would be lost: work committed and pushed, ROADMAP and docs up
  to date, and anything worth remembering saved in memory.
