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

```sh
cargo build
cargo test
cargo run -p r3d-app     # desktop app
cargo run -p r3d-cli     # headless CLI
```

## Documentation

- [`docs/architecture.md`](docs/architecture.md): crates, data flow, threading.
- [`docs/physics.md`](docs/physics.md): cloth solver and fabric model.
- [`docs/ux.md`](docs/ux.md): UX and visual design principles.
- [`docs/specs/`](docs/specs/): one spec per feature.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): including the **clean-room policy**.

## License

MIT, see [`LICENSE`](LICENSE) and
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
