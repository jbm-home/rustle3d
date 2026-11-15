#!/usr/bin/env bash
# M4–M7 demo: a body, the sewn A-line skirt template placed around it, a
# seam report, headless drapes in cotton and denim, a pin holding the hem,
# renders of each; then the same live in the app (Simulate, drag to pull,
# option-click to pin).
# Run from the repository root: ./demos/m4/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(mktemp -d)/Skirt.r3d"

echo "== 1. A standard body (fitted MakeHuman avatar), measurements achieved"
"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m4/body.json | grep -E '"(height_cm|bust_cm|waist_cm|hips_cm)"' | head -n 4

echo "== 2. The skirt template: three pieces, placed and sewn; seam lengths"
"$R3D" run "$OUT" templates/a-line-skirt.json | grep -E '"(length_mm|mismatch_mm)"' | head -n 9

echo "== 3. Drape it headless (5 s of simulated time), in cotton then denim"
"$R3D" simulate "$OUT" --frames 300 | grep -E '"(top_m|bottom_m|max_seam_gap_mm|ms_per_frame)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../cotton.png" >/dev/null
"$R3D" run "$OUT" <(echo '[{"type": "garment.set_fabric", "fabric": "denim"}]') >/dev/null
"$R3D" simulate "$OUT" --frames 300 >/dev/null
"$R3D" render "$OUT" --view 3d --out "$OUT/../denim.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern.png" >/dev/null

echo "== 4. Pin the middle of the front hem out in front of the knees"
"$R3D" run "$OUT" <(echo '[{"type": "pin.add", "panel": "panel:2", "at": [390, 0], "position": [0.05, 0.62, 0.32]}]') >/dev/null
"$R3D" simulate "$OUT" --frames 240 >/dev/null
"$R3D" render "$OUT" --view 3d --out "$OUT/../pinned.png" >/dev/null
ls -1 "$(dirname "$OUT")"/*.png

echo
echo "Live in the app: (cd app && flutter run -d macos --release), then Simulate (Space),"
echo "drag the cloth to pull it, option-click to pin it."
