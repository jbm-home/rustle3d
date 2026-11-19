#!/usr/bin/env bash
# M1 demo: create a project, build an A-line skirt pattern with a command
# script (including undo/redo and an atomic batch), query it, and show a
# structured error. Run from the repository root: ./demos/m1/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

R3D=(cargo run --quiet -p r3d-cli --)
OUT="$(demo_out m1)/Skirt.r3d"

echo "== 1. New project"
"${R3D[@]}" new "$OUT"

echo "== 2. Run the skirt script"
"${R3D[@]}" run "$OUT" demos/m1/skirt.json | tail -n 40

echo "== 3. Query the saved project"
"${R3D[@]}" query "$OUT" '{"type": "panel.list"}'

echo "== 4. A failing script changes nothing (exit code 1, structured error)"
echo '[{"type": "panel.delete", "panel": "panel:2"}]' \
  | "${R3D[@]}" run "$OUT" - || echo "(exit code $?)"

echo "== 5. Discover the API"
"${R3D[@]}" schema | head -n 20

echo "Project written to $OUT"
