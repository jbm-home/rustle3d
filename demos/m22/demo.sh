#!/usr/bin/env bash
# M22 demo: grading and sizes. An agent (or a script) sets the women's
# size range 36 to 52 (drawn in 40), drafts a shirt (graded to every size
# by the template), checks that every seam still matches in every size,
# drapes sizes 36, 40 and 52 headless, each on an avatar with that size's
# measurements, and exports the graded nest as PDF and DXF-AAMA.
# Run from the repository root: ./demos/m22/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/Shirt.r3d"

"$R3D" new "$OUT" >/dev/null
cat > "$DIR/graded.json" <<'JSON'
[
  {"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"},
  {"type": "template.apply", "template": "shirt"}
]
JSON
"$R3D" run "$OUT" "$DIR/graded.json" >/dev/null

echo "== 1. The sizes and their bodies (bust cm)"
"$R3D" query "$OUT" '{"type": "grading.get"}' \
  | grep -E '"(name|bust_cm|base|graded_points)"' | grep -v '"bust_cm": -\?[0-9.]*,$' | tr -d ' ,' | paste -sd' '
echo "== 2. Every seam matches in every size"
"$R3D" query "$OUT" '{"type": "grading.check"}' | grep '"ok"'

for size in 36 40 52; do
  echo "== 3. Size $size: drape it on its body (3 s), then a photo"
  echo "[{\"type\": \"grading.show_size\", \"size\": \"$size\"}]" > "$DIR/show.json"
  "$R3D" run "$OUT" "$DIR/show.json" >/dev/null
  "$R3D" simulate "$OUT" --frames 180 | grep -E '"(max_seam_gap_mm|settled)"' | tr -d ' ,' | paste -sd' '
  "$R3D" render "$OUT" --view photo --quality quick --camera front --frame body \
    --out "$DIR/shirt-$size.png" >/dev/null
done

echo "== 4. The graded nest"
"$R3D" export "$OUT" --format pdf --paper single --sizes all | grep '"path"'
"$R3D" export "$OUT" --format dxf --sizes all | grep '"path"'
ls -1 "$DIR"/*.png
