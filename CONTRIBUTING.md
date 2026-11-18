# Contributing to Rustle3D

Thanks for your interest! Rustle3D is licensed under the GNU General Public
License, version 3 or (at your option) any later version (GPL-3.0-or-later,
see `LICENSE`). By contributing, you agree that your contributions are
licensed under the same terms.

## Clean-room policy (please read first)

Rustle3D is an independent, clean-room implementation of a 3D garment design
tool. To keep the project legally clean, every contributor must follow these
rules:

1. Do **not** use, run, decompile or inspect proprietary garment software
   (e.g. CLO3D, Marvelous Designer, Browzwear, Style3D, Optitex) or its file
   formats, binaries, assets or material libraries while working on Rustle3D.
   Do not submit material derived from them: screenshots, UI layouts,
   terminology, parameter tables, sample files.
2. Specs and code come from public academic papers, open standards
   (DXF-AAMA/ASTM, glTF, OBJ, SVG, PDF), general garment-making knowledge and
   the project maintainers' own descriptions. Each spec in `docs/specs/`
   records its sources on a `Provenance:` line.
3. Use our own UI layout, icons, colors and names. Do not put third-party
   trademarks in product names, UI strings or code identifiers.
4. No support for proprietary file formats obtained by reverse engineering.
   Interoperability goes through open formats.
5. Body models and other assets must have a license that allows commercial
   redistribution (e.g. CC0). SMPL/SMPL-X and other non-commercial models
   are not accepted.
6. If you copy or closely adapt code from another project, its license must
   be compatible with GPL-3.0-or-later (never GPL-2.0-only); keep its
   copyright notice and add it to `THIRD_PARTY_NOTICES.md`.
   Prefer reimplementing from the original papers, and cite them.

Pull requests that do not respect these rules will be closed.

## Development

Requirements: a recent stable Rust toolchain (`rustup` picks it up from
`rust-toolchain.toml`).

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Engineering rules (also in `CLAUDE.md`):

- No `unwrap()`/`expect()`/`panic!` in non-test code; use proper error types
  (`thiserror`). The app must never crash on user input.
- Every user-visible operation is a command in `r3d-commands`, with
  validation, a JSON schema and an undo/redo test. No business logic in the UI.
- Tests: unit tests, property tests for geometry, golden files for
  serialization, deterministic simulation regression tests.
- New dependencies must have a license compatible with GPL-3.0-or-later,
  preferably a permissive one (MIT, Apache-2.0, BSD, Zlib, ISC, CC0).

## Pull requests

- One feature or fix per PR, small commits with clear messages.
- If the PR implements or changes a feature, update its spec in `docs/specs/`.
- CI (Linux, Windows, macOS) must be green: fmt, clippy, tests.
