#!/usr/bin/env bash
# M8 demo: alter a draped garment with the 3D tools' commands. The skirt is
# shortened by 30 mm (edge.lengthen on the front hem moves the whole
# hemline) and each side seam is taken in by 30 mm at the waist (seam.take_in
# at the waist end of one side seam; the back is a linked mirror, so the
# other side seam follows: 60 mm in all). The pattern, its 1:1 PDF
# and the drape follow. In the app the same edits are made in 3D:
# shift-drag the hem, shift-drag a seam near the waist.
# Run from the repository root: ./demos/m8/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(mktemp -d)/Skirt.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m4/body.json >/dev/null
"$R3D" run "$OUT" templates/a-line-skirt.json >/dev/null

echo "== 1. The skirt as drafted: drape and pattern"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../before.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern-before.png" >/dev/null

echo "== 2. Shorten 30 mm; take in each side seam 30 mm at the waist; seams stay matched"
"$R3D" run "$OUT" demos/m8/alter.json | grep -E '"mismatch_mm"'

echo "== 3. The drape and the pattern follow"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../after.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern-after.png" >/dev/null
"$R3D" export "$OUT" --format pdf >/dev/null
ls -1 "$(dirname "$OUT")"/*.png "$OUT"/exports/*.pdf
