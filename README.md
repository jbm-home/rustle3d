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

### Installing Flutter

The app needs Flutter ≥ 3.47 (Dart ≥ 3.14, see `app/pubspec.lock`). On
macOS and Linux, a clone of the stable channel is the simplest install:

```sh
sudo apt install git curl unzip xz-utils zip   # Linux only
git clone --depth 1 -b stable https://github.com/flutter/flutter.git ~/development/flutter
echo 'export PATH="$HOME/development/flutter/bin:$PATH"' >> ~/.zshrc   # or ~/.bashrc
export PATH="$HOME/development/flutter/bin:$PATH"
flutter --disable-analytics
flutter doctor          # only the line for your desktop OS has to be ✓
```

`flutter doctor` also lists Android, Chrome and Android Studio; they are
not needed. Update later with `flutter upgrade`.

### Linux (and WSL2 on Windows)

Install Flutter as above, then the desktop toolchain:

```sh
sudo apt install build-essential clang cmake ninja-build pkg-config \
  libgtk-3-dev mesa-vulkan-drivers libvulkan1
(cd app && flutter run -d linux)
```

Under WSL2, clone the repository inside the Linux file system (e.g.
`~/Git`), not under `/mnt/c`: builds there are much slower.

### Windows (native)

Run everything from Windows itself: Flutter inside WSL only builds the
Linux app. Prerequisites:

- Visual Studio 2022 (Community is enough) with the "Desktop development
  with C++" workload.
- Rust through `rustup-init.exe`, default MSVC toolchain
  (`stable-x86_64-pc-windows-msvc`).
- Flutter: `git clone --depth 1 -b stable https://github.com/flutter/flutter.git C:\dev\flutter`,
  add `C:\dev\flutter\bin` to `PATH`; `flutter doctor` must show ✓ for
  Visual Studio.
- Git for Windows (Git Bash runs `scripts/release.sh`).

Clone the repository on the Windows side (e.g. `C:\dev\rustle3d`), not
under `\\wsl$\`, then:

```powershell
cd C:\dev\rustle3d\app
flutter run -d windows
```

The GPU is used natively, so the 3D view is much faster than under WSL2.

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
