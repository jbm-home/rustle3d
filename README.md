<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-dark.svg">
    <img src="assets/brand/logo.svg" alt="Rustle3D" height="72">
  </picture>
</p>

<p align="center">
  <strong>3D garment design and cloth simulation, written in Rust.</strong><br>
  Draft sewing patterns in 2D, sew them, and drape them on an avatar with real-time cloth physics.
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
  <img alt="Platforms: Windows, macOS, Linux" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey">
  <img alt="Rust stable" src="https://img.shields.io/badge/rust-stable-orange">
</p>

---

Rustle3D is a free desktop app for making clothes: draft the pattern in
2D, sew the pieces together, and watch the garment drape on a body in 3D,
with a cloth simulation in real time. It covers the core workflow of
professional 3D fashion tools with a simpler, more approachable
interface, on macOS, Windows and Linux.

- [Download](#download)
- [Features](#features)
- [Scripts and AI agents](#scripts-and-ai-agents)
- [Building from source and contributing](#building-from-source-and-contributing)
- [License](#license)

## Download

Get the latest version from [rustle3d.com](https://rustle3d.com) or the
[releases page](https://github.com/jbm-home/rustle3d/releases/latest):

| System | Download |
|---|---|
| **macOS** 12 or later, Intel and Apple silicon | `Rustle3D-<version>-macos-universal.dmg`: open it and drag Rustle3D onto Applications |
| **Windows** 10 or 11, x64 (Windows on Arm runs it too) | `Rustle3D-<version>-windows-x64-setup.exe`: the installer, no administrator rights needed |
| **Linux** x64, with GTK 3 and Vulkan | `Rustle3D-<version>-linux-x64.tar.gz`: unpack it and run `./rustle3d` |

The packages are not signed yet, so the first launch needs one extra
confirmation: the [user guide](docs/user-guide.md#installing) shows how,
system by system. The user guide is also the place to learn the app, from
the first template to the photo of the finished garment.

The 3D view needs a graphics card with Metal (macOS), Direct3D 12
(Windows) or Vulkan (Linux); any card from the last ten years should do.

## Features

| Area | What you can do |
|---|---|
| **Patterns (2D)** | Start from templates drafted to the body's measurements (T-shirt, shirt, dress, skirts, trousers, tote bag) or draw pieces with polygon, curve and cut tools. Constraints, dimensions, formulas (`waist / 4 + 10 mm`), darts, pleats, holes, symmetric half pieces, notches and internal lines. |
| **Sewing** | Sew edges together, see direction and length mismatches, gather, add ease, match both sides in one click. Buttons and zippers close the garment. |
| **Drape (3D)** | Real-time XPBD cloth simulation on the GPU, with collisions, self-collision, pins and pulling the cloth with the mouse. Several garments in layers. Fabric presets (cotton, denim, jersey, silk, wool) and prints that follow the grainline. |
| **Avatars** | A parametric body set by its measurements, imported scans and characters (OBJ, glTF), walking motions and captured motions (BVH). |
| **Sizes and fit** | Grading over a size range, each size draped on a matching body; fit maps and a virtual tape measure. |
| **Production** | Markers with yardage and cost, a tech pack PDF, a projector mode for cutting on the table. |
| **Images** | Path-traced studio photos of the draped garment. |
| **Exchange** | Open formats only: patterns in SVG, PDF (1:1, tiled), DXF-AAMA; DXF, SVG and PDF import; the draped garment in glTF/GLB and OBJ, and game-ready exports (LODs, normal maps, skinning). |
| **Automation** | A headless CLI (`r3d`) and an MCP server, headless or attached live to the open app. |

The app is in English and French.

## Scripts and AI agents

Everything the app does is also a command, which scripts and AI agents
can run. Each package comes with `r3d`, the command-line tool:

```sh
r3d new Skirt.r3d                                  # a new project
echo '[{"type": "template.apply", "template": "a_line_skirt"}]' | r3d run Skirt.r3d -
r3d simulate Skirt.r3d                             # drape it, without the app
r3d render Skirt.r3d --view photo --out skirt.png  # a studio photo of it
r3d mcp --project Skirt.r3d                        # serve it to an AI agent over MCP
```

The app can also let an agent work on the project open on screen (an
option in the app, off by default). Every command and its parameters:
[`docs/commands.md`](docs/commands.md).

## Building from source and contributing

Rustle3D is written in Rust (the core: geometry, cloth simulation,
rendering) with a Flutter desktop interface.
[`docs/building.md`](docs/building.md) walks through installing the
tools on each system, building and running the app, and the project's
layout. Contributions are welcome: read
[`CONTRIBUTING.md`](CONTRIBUTING.md) first, including the clean-room
policy, and see [`ROADMAP.md`](ROADMAP.md) for what is coming.

## License

GNU General Public License v3.0 or later (GPL-3.0-or-later), see
[`LICENSE`](LICENSE) and [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
Rustle3D is free to use for any purpose, including commercial work; the
garments, patterns and images you make with it are yours, under any terms
you choose. If you distribute a modified version of Rustle3D, it must stay
under the GPL with its source code available. Contributions are welcome
under the [Contributor License Agreement](CLA.md), see
[`CONTRIBUTING.md`](CONTRIBUTING.md).
