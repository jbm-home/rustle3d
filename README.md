# Rustle3D

3D garment design and cloth simulation, written in Rust.

Draft sewing patterns in 2D, sew them together and drape them on an avatar
with real-time cloth physics. Rustle3D aims to cover the core workflow of
professional 3D fashion tools with a simpler, more approachable UI, and to be
fully drivable by AI agents through an MCP server.

> **Status:** pre-alpha. The workspace skeleton and planning documents are in
> place; see [`ROADMAP.md`](ROADMAP.md) for what comes next.

## Planned features (v0.1)

- 2D pattern editor: panels, Bézier contours, symmetry, notches, internal
  lines, snapping, undo/redo.
- Sewing between edges, with direction and length mismatch shown clearly.
- Arrangement points around the avatar and automatic pre-positioning.
- Real-time drape (XPBD) with avatar collision and self-collision.
- Fabric presets: cotton, denim, jersey, silk, wool.
- Open formats only: project folders with JSON, OBJ/glTF, SVG/PDF patterns
  at 1:1 scale.
- A headless CLI and an MCP server for AI agents, including offscreen renders.

## Building

The core is a Rust workspace; the desktop app is Flutter (`app/`) over it.

```sh
cargo build && cargo test                  # core, CLI, MCP server
cargo run -p r3d-cli -- --help             # headless CLI
./scripts/check.sh                         # everything CI checks
(cd app && flutter run -d macos)           # desktop app (or -d linux)
```

Prerequisites: Rust stable (`rustup`), Flutter stable, and
`cargo install flutter_rust_bridge_codegen --version 2.13.0` (to regenerate
the Dart bindings). macOS needs Xcode.

### Linux (and WSL2 on Windows)

```sh
sudo apt install build-essential clang cmake ninja-build pkg-config \
  libgtk-3-dev mesa-vulkan-drivers libvulkan1
(cd app && flutter run -d linux)
```

On Linux and Windows the views are rendered by the GPU and copied to the
UI (shared textures are macOS-only for now). Under WSL2 the app runs
through WSLg; the GPU is Mesa's Vulkan driver (or its software fallback),
so expect lower frame rates than native.

### Release packages

`./scripts/release.sh` builds the app and the CLI in release mode for the
current OS and packages them in `dist/` (unsigned: a zip of `Rustle3D.app`
on macOS, a tarball of the app bundle on Linux, a zip on Windows from Git
Bash). The manual GitHub workflow "Release builds" runs it on all three.

## Documentation

- [`docs/architecture.md`](docs/architecture.md): crates, data flow, threading.
- [`docs/physics.md`](docs/physics.md): cloth solver and fabric model.
- [`docs/ux.md`](docs/ux.md): UX and visual design principles.
- [`docs/specs/`](docs/specs/): one spec per feature.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): including the **clean-room policy**.

## License

MIT, see [`LICENSE`](LICENSE) and
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
