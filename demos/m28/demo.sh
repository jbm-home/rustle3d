#!/usr/bin/env bash
# M28 demo: parametric patterns.
# A quarter skirt is drafted with formulas: its waist is `waist / 4 +
# 10 mm`, its hem `hips / 4 + ease`. It redrafts itself when the body
# changes, and when the size range is set, every size is drafted with its
# own measurements. A wrong formula is refused with a code.
# Run from the repository root: ./demos/m28/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
P="$DIR/Skirt.r3d"
"$R3D" new "$P" >/dev/null

body() { echo "{\"masculinity\": 0, \"height_cm\": 168, \"bust_cm\": 90, \"waist_cm\": $1, \"hips_cm\": $2}"; }

cat > "$DIR/skirt.json" <<JSON
[
  {"type": "avatar.set", "body": $(body 72 98)},
  {"type": "panel.create_polygon", "name": "Front", "points": [[0, 0], [260, 0], [200, 600], [0, 600]]},
  {"type": "variable.set", "name": "ease", "formula": "4 cm"},
  {"type": "constraint.add", "constraint": {"kind": "fixed", "point": "\$1.created[1]"}},
  {"type": "constraint.add", "constraint": {"kind": "horizontal", "edge": "\$1.created[5]"}},
  {"type": "constraint.add", "constraint": {"kind": "horizontal", "edge": "\$1.created[7]"}},
  {"type": "constraint.add", "constraint": {"kind": "vertical", "edge": "\$1.created[8]"}},
  {"type": "constraint.add", "constraint": {"kind": "length", "edge": "\$1.created[8]", "mm": 600}},
  {"type": "constraint.add", "constraint": {"kind": "length", "edge": "\$1.created[7]", "mm": 1,
   "formula": "waist / 4 + 10 mm"}},
  {"type": "constraint.add", "constraint": {"kind": "length", "edge": "\$1.created[5]", "mm": 1,
   "formula": "hips / 4 + ease"}},
  {"type": "panel.get", "panel": "\$1.created[0]"},
  {"type": "avatar.set", "body": $(body 80 104)},
  {"type": "panel.get", "panel": "\$1.created[0]"}
]
JSON
echo "== 1. A quarter skirt: waist = waist / 4 + 10 mm, hem = hips / 4 + ease"
"$R3D" run "$P" "$DIR/skirt.json" > "$DIR/skirt.out"
python3 - "$DIR/skirt.out" <<'PY'
import json, sys
r = [s["result"] for s in json.load(open(sys.argv[1]))["results"]]
def widths(g): e = g["edges"]; return f'waist {e[2]["length_mm"]:.1f} mm, hem {e[0]["length_mm"]:.1f} mm'
print("waist 72 cm, hips 98 cm: ", widths(r[10]))
print("== 2. Another body (waist 80, hips 104): the skirt redrafts itself")
print("waist 80 cm, hips 104 cm:", widths(r[12]))
PY

echo "== 3. Sizes 36-44: each drafted with its own measurements"
echo '[{"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}]' > "$DIR/sizes.json"
"$R3D" run "$P" "$DIR/sizes.json" >/dev/null
for size in 36 38 40 42 44; do
  "$R3D" query "$P" "{\"type\": \"panel.get\", \"panel\": \"panel:2\", \"size\": \"$size\"}" \
    | python3 -c "import json,sys; e=json.load(sys.stdin)['edges']; print(f'size $size: waist {e[2][\"length_mm\"]:.1f} mm, hem {e[0][\"length_mm\"]:.1f} mm, length {e[3][\"length_mm\"]:.0f} mm')"
done

echo "== 4. A formula with a wrong unit is refused"
echo '[{"type": "variable.set", "name": "ease", "formula": "waist + 2"}]' > "$DIR/wrong.json"
"$R3D" run "$P" "$DIR/wrong.json" 2>&1 | grep -o '"code": *"[a-z_.]*"' | head -1 || true
