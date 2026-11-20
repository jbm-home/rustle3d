#!/usr/bin/env bash
# Seamly2D/Valentina import (spec 28).
# A quarter skirt drafted in Valentina's format, with its measurement
# file, comes in drafted for those measurements; graded on the women's
# chart, each size is drafted with its own body; drafted again for
# another avatar, it follows. A multisize measurement file then gives a
# body and a size range. The files are the test files written by hand
# for Rustle3D (crates/r3d-commands/tests/data/parametric/).
# Run from the repository root: ./demos/m28/seamly.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DATA=crates/r3d-commands/tests/data/parametric
DIR="$(demo_out m28-seamly)"
P="$DIR/Skirt.r3d"
"$R3D" new "$P" >/dev/null

width() {
  "$R3D" query "$P" "{\"type\": \"panel.get\", \"panel\": \"panel:2\"$1}" \
    | python3 -c "import json,sys; e=json.load(sys.stdin)['edges']; print(f'waist {e[3][\"length_mm\"]:.1f} mm, hem {e[0][\"length_mm\"]:.1f} mm, length {e[4][\"length_mm\"]:.0f} mm')"
}

echo "== 1. Import skirt.val: its measurement file body.vit comes along"
"$R3D" import "$P" "$DATA/skirt.val"
echo "front: $(width '')"

echo "== 2. Sizes 36-44: the pattern is run for each size's body"
echo '[{"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}]' > "$DIR/sizes.json"
"$R3D" run "$P" "$DIR/sizes.json" >/dev/null
for size in 36 38 40 42 44; do
  echo "size $size: $(width ", \"size\": \"$size\"")"
done

echo "== 3. Another avatar (waist 80, hips 104): Draft again for the avatar"
cat > "$DIR/redraft.json" <<'JSON'
[
  {"type": "avatar.set", "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 92,
   "waist_cm": 80, "hips_cm": 104}},
  {"type": "garment.redraft", "garment": "garment:1"}
]
JSON
"$R3D" run "$P" "$DIR/redraft.json" >/dev/null
echo "front: $(width '')"
"$R3D" render "$P" --view board --out "$DIR/skirt.png" >/dev/null
echo "board: $DIR/skirt.png"

echo "== 4. A multisize measurement file: the body and sizes 40-48"
Q="$DIR/Sizes.r3d"
"$R3D" new "$Q" >/dev/null
"$R3D" measurements "$Q" "$DATA/sizes.vst"
