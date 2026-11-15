#!/usr/bin/env bash
# Everything CI checks, locally. Run before every commit; any failure stops it.
# (docs/commands.md freshness is checked by a test in r3d-commands.)
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt";    cargo fmt --all --check
echo "== cargo clippy"; cargo clippy --workspace --all-targets -- -D warnings
echo "== cargo test";   cargo test --workspace --quiet
if command -v flutter >/dev/null; then
  echo "== flutter analyze"; (cd app && flutter analyze --no-pub)
fi
echo "All checks passed."
