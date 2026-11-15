#!/usr/bin/env bash
# Everything CI checks, locally. Run before every commit; any failure stops it.
# (docs/commands.md freshness is checked by a test in r3d-commands.)
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt";    cargo fmt --all --check
echo "== cargo clippy"; cargo clippy --workspace --all-targets -- -D warnings
echo "== cargo test";   cargo test --workspace --quiet
if command -v flutter_rust_bridge_codegen >/dev/null; then
  # Stale bindings compile until someone regenerates them: check they are
  # up to date with crates/r3d-ffi/src/api.
  echo "== flutter bindings"
  (cd app && flutter_rust_bridge_codegen generate >/dev/null 2>&1)
  git diff --quiet -- app/lib/src/rust crates/r3d-ffi/src/frb_generated.rs \
    || { echo "Flutter bindings were stale (now regenerated): review and commit them"; exit 1; }
fi
if command -v flutter >/dev/null; then
  echo "== flutter analyze"; (cd app && flutter analyze --no-pub)
fi
echo "All checks passed."
