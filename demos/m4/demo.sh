#!/usr/bin/env bash
# M4 demo (in progress): a body, the sewn A-line skirt template placed
# around it, a seam report, then the app drapes it with real-time physics.
# Run from the repository root: ./demos/m4/demo.sh
set -euo pipefail

cargo build --quiet -p r3d-cli
R3D=./target/debug/r3d
OUT="$(mktemp -d)/Skirt.r3d"

echo "== 1. A standard body (fitted MakeHuman avatar), measurements achieved"
"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m4/body.json | grep -E '"(height_cm|bust_cm|waist_cm|hips_cm)"' | head -n 4

echo "== 2. The skirt template: three pieces, placed and sewn; seam lengths"
"$R3D" run "$OUT" templates/a-line-skirt.json | grep -E '"(length_mm|mismatch_mm)"' | head -n 9

echo "== 3. Fabrics: switch to denim"
"$R3D" run "$OUT" <(echo '[{"type": "garment.set_fabric", "fabric": "denim"}]') >/dev/null
"$R3D" query "$OUT" '{"type": "project.summary"}' | grep '"fabric"'

echo
echo "Drape it in the app: (cd app && flutter run -d macos), then Simulate (Space)."
