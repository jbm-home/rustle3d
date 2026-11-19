#!/usr/bin/env bash
# M29 demo: construction details.
# An A-line skirt gets two darts and a stitched knife pleat at the front
# waist, and a patch pocket sewn on its placement line (a seam between a
# run of the pocket's edges and an internal line). It drapes headless;
# then a hole is cut and two pieces are joined.
# Run from the repository root: ./demos/m29/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(demo_out m29)"
P="$DIR/Skirt.r3d"
"$R3D" new "$P" >/dev/null

cat > "$DIR/skirt.json" <<'JSON'
[
  {"type": "avatar.set", "preset": "Women M",
   "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}},
  {"type": "template.apply", "template": "a_line_skirt", "sizes": {"length_cm": 60, "waist_cm": 79}},
  {"type": "panel.list"}
]
JSON
"$R3D" run "$P" "$DIR/skirt.json" > "$DIR/skirt.out"
# The front's waist (its third edge) and where to put things on it.
read -r PANEL WAIST LEN CX TOP < <(python3 - "$DIR/skirt.out" "$R3D" "$P" <<'PY'
import json, subprocess, sys
r = [s["result"] for s in json.load(open(sys.argv[1]))["results"]]
front = next(p for p in r[2] if p["name"] == "Front")
g = json.loads(subprocess.check_output([sys.argv[2], "query", sys.argv[3],
    json.dumps({"type": "panel.get", "panel": front["id"]})]))
w = g["edges"][2]
cx = (g["bbox"]["min"][0] + g["bbox"]["max"][0]) / 2 + 60
print(front["id"], w["id"], w["length_mm"], cx, g["bbox"]["max"][1] - 180)
PY
)

cat > "$DIR/details.json" <<JSON
[
  {"type": "dart.add", "edge": "$WAIST", "at_mm": $(python3 -c "print(0.8 * $LEN)"), "width_mm": 16, "length_mm": 100},
  {"type": "dart.add", "edge": "$WAIST", "at_mm": $(python3 -c "print(0.2 * $LEN)"), "width_mm": 16, "length_mm": 100},
  {"type": "internal_line.add", "panel": "$PANEL", "start": [$(python3 -c "print($CX - 60)"), $TOP],
   "segments": [{"kind": "line", "to": [$(python3 -c "print($CX - 60)"), $(python3 -c "print($TOP - 130)")]},
                {"kind": "line", "to": [$(python3 -c "print($CX + 60)"), $(python3 -c "print($TOP - 130)")]},
                {"kind": "line", "to": [$(python3 -c "print($CX + 60)"), $TOP]}]},
  {"type": "panel.create_rect", "name": "Pocket", "origin": [$(python3 -c "print($CX + 400)"), $(python3 -c "print($TOP - 130)")],
   "width": 120, "height": 140},
  {"type": "seam.add", "a": {"edges": ["\$3.created[8]", "\$3.created[5]", "\$3.created[6]"], "from_mm": 10, "to_mm": 390},
   "b": {"line": "\$2.created[0]"}},
  {"type": "panel.place", "panel": "\$3.created[0]", "angle_deg": 0, "top_cm": 89, "offset_mm": 40},
  {"type": "seam.list"}
]
JSON
echo "== 1. Two darts and a patch pocket sewn on its placement line"
"$R3D" run "$P" "$DIR/details.json" > "$DIR/details.out"
python3 - "$DIR/details.out" <<'PY'
import json, sys
seams = json.load(open(sys.argv[1]))["results"][-1]["result"]
for s in seams[-3:]:
    kind = lambda x: x.get("edge") or (f'line {x["line"]}' if "line" in x else f'{len(x["edges"])} edges')
    print(f'seam {s["id"]}: {kind(s["a"])} ({s["a"]["length_mm"]} mm) to {kind(s["b"])} ({s["b"]["length_mm"]} mm), mismatch {s["mismatch_mm"]} mm')
PY

echo "== 2. A knife pleat in the middle of the front waist, stitched 6 cm"
MIDDLE=$(python3 - "$R3D" "$P" "$PANEL" <<'PY'
import json, subprocess, sys
g = json.loads(subprocess.check_output([sys.argv[1], "query", sys.argv[2],
    json.dumps({"type": "panel.get", "panel": sys.argv[3]})]))
# The waist between the darts: the longest top edge left.
top = max(g["edges"], key=lambda e: (min(e["start"]["pos"][1], e["end"]["pos"][1]), e["length_mm"]))
print(top["id"], top["length_mm"])
PY
)
read -r MID MLEN <<< "$MIDDLE"
echo "[{\"type\": \"pleat.add\", \"edge\": \"$MID\", \"at_mm\": $(python3 -c "print($MLEN / 2 - 15)"), \"depth_mm\": 15, \"stitched_mm\": 60}]" > "$DIR/pleat.json"
"$R3D" run "$P" "$DIR/pleat.json" | grep -c '"seam:' | sed 's/^/new seams: /'

echo "== 3. The skirt drapes (headless, 180 frames)"
"$R3D" simulate "$P" --frames 180 2>/dev/null | python3 -c "import json,sys; r=json.load(sys.stdin); print({k: r[k] for k in r if k in ('frames','max_seam_gap_mm','particles')})" || true
"$R3D" render "$P" --view 3d --out "$DIR/skirt.png" >/dev/null && echo "3D view: $DIR/skirt.png"

echo "== 4. A hole, and two pieces joined"
cat > "$DIR/shapes.json" <<'JSON'
[
  {"type": "panel.create_rect", "name": "Yoke", "origin": [3000, 0], "width": 300, "height": 200},
  {"type": "panel.create_rect", "origin": [3250, 50], "width": 150, "height": 100},
  {"type": "panel.boolean", "panel": "$0.created[0]", "with": "$1.created[0]", "op": "union"},
  {"type": "panel.add_hole", "panel": "$0.created[0]", "start": [3080, 80],
   "segments": [{"kind": "line", "to": [3140, 80]}, {"kind": "line", "to": [3140, 120]}, {"kind": "line", "to": [3080, 120]}]},
  {"type": "panel.get", "panel": "$0.created[0]"}
]
JSON
"$R3D" run "$P" "$DIR/shapes.json" | python3 -c "import json,sys; g=json.load(sys.stdin)['results'][-1]['result']; print(f'yoke: {g[\"point_count\"]} points, {g[\"area_mm2\"]:.0f} mm2 of cloth (300x200 + 100x100 - 60x40)')"
