#!/usr/bin/env bash
# Everything CI checks, locally. Run before every commit; any failure stops it.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt";    cargo fmt --all --check
echo "== cargo clippy"; cargo clippy --workspace --all-targets -- -D warnings
echo "== cargo test";   cargo test --workspace --quiet
echo "== docs";         cargo xtask gen-docs >/dev/null && git diff --quiet docs/commands.md \
                          || { echo "docs/commands.md was out of date (now regenerated)"; exit 1; }
if command -v flutter >/dev/null; then
  echo "== flutter analyze"; (cd app && flutter analyze --no-pub)
fi
echo "All checks passed."
