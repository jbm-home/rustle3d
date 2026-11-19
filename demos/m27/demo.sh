#!/usr/bin/env bash
# M27 demo: sketching with constraints and symmetry.
# A script draws half a bodice against a centre front axis (the other half
# follows), fixes its dimensions with constraints, then changes the bust
# width: the piece and its other half are redrawn, the constraints still
# hold. Then a circle skirt is drawn from arc sectors given the waist and
# the length. The board is rendered to PNG.
# Run from the repository root: ./demos/m27/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(demo_out m27)"
P="$DIR/Bodice.r3d"
"$R3D" new "$P" >/dev/null

# Half a bodice, counter-clockwise from the centre front waist: side waist,
# underarm, shoulder point, neck point, centre front neck. The closing edge
# (centre front neck down to the waist) is the axis.
cat > "$DIR/bodice.json" <<'JSON'
[
  {"type": "panel.create_polygon", "name": "Front", "symmetric": true,
   "points": [[0, 0], [220, 0], [240, 250], [180, 400], [70, 440], [0, 380]]},
  {"type": "guide.add", "a": [-400, 250], "b": [400, 250]},
  {"type": "constraint.add", "constraint": {"kind": "fixed", "point": "$0.created[1]"}},
  {"type": "constraint.add", "constraint": {"kind": "horizontal", "edge": "$0.created[7]"}},
  {"type": "constraint.add", "constraint": {"kind": "distance", "a": "$0.created[1]", "b": "$0.created[3]",
   "mm": 240, "along": "horizontal"}},
  {"type": "constraint.add", "constraint": {"kind": "distance", "a": "$0.created[1]", "b": "$0.created[3]",
   "mm": 250, "along": "vertical"}},
  {"type": "constraint.add", "measure": true, "constraint": {"kind": "length", "edge": "$0.created[10]", "mm": 1}},
  {"type": "constraint.add", "constraint": {"kind": "parallel", "a": "$0.created[7]", "b": "$0.created[11]"}},
  {"type": "panel.get", "panel": "$0.created[0]"},
  {"type": "constraint.set_value", "constraint": "$4.created[0]", "value": 265},
  {"type": "panel.get", "panel": "$0.created[0]"},
  {"type": "constraint.list", "panel": "$0.created[0]"}
]
JSON
echo "== 1. Half a bodice against an axis, with fixed dimensions"
"$R3D" run "$P" "$DIR/bodice.json" > "$DIR/bodice.out"
python3 - "$DIR/bodice.out" <<'PY'
import json, sys
r = [s["result"] for s in json.load(open(sys.argv[1]))["results"]]
before, after, lst = r[8], r[10], r[11]
def span(g): return f'x {g["bbox"]["min"][0]:.0f} .. {g["bbox"]["max"][0]:.0f} mm, {g["point_count"]} points'
print("drawn:", span(before), "| symmetric:", before["symmetry"] is not None)
print("== 2. Bust width 240 -> 265 mm: the piece and its other half follow")
print("now:  ", span(after))
for c in lst["constraints"]:
    v = c.get("mm", c.get("deg", ""))
    v = round(v, 1) if isinstance(v, float) else v
    print(f'  {c["kind"]:<12} {c.get("along", ""):<10} {v!s:<8} holds: {c["holds"]}')
g = lst["groups"][0]
print("degrees of freedom left:", g["freedom"], "| over-constrained:", bool(g["redundant"]))
PY

echo "== 3. A circle skirt: two half circles for a 72 cm waist, 60 cm long"
cat > "$DIR/skirt.json" <<'JSON'
[
  {"type": "panel.create_sector", "name": "Skirt front", "centre": [1200, 0],
   "inner_length_mm": 360, "length_mm": 600, "sweep_deg": 180},
  {"type": "panel.create_sector", "name": "Skirt back", "centre": [2900, 0],
   "inner_length_mm": 360, "length_mm": 600, "sweep_deg": 180},
  {"type": "panel.get", "panel": "$0.created[0]"}
]
JSON
"$R3D" run "$P" "$DIR/skirt.json" > "$DIR/skirt.out"
python3 - "$DIR/skirt.out" <<'PY'
import json, sys
g = json.load(open(sys.argv[1]))["results"][2]["result"]
e = g["edges"]
print(f'waist arc {sum(x["length_mm"] for x in e[3:5]):.1f} mm, hem {sum(x["length_mm"] for x in e[0:2]):.0f} mm, side {e[2]["length_mm"]:.0f} mm')
PY

"$R3D" render "$P" --view board --out "$DIR/board.png" >/dev/null
echo "board: $DIR/board.png"
