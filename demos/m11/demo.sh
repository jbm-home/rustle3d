#!/usr/bin/env bash
# M11 demo: drafting from scratch. An agent (or a script) draws a bodice
# outline as a polygon, bends its armhole into a curve, adds notches, sets
# the shoulder width, and gathers a skirt into a waistband that drapes.
# In the app: the Polygon (P), Curve (C) and Notch (N) tools, corners
# picked with Select (⇧-click a second for the distance), and the
# Gathered switch on a seam. Run from the repository root.
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(mktemp -d)/Draft.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m10/body.json >/dev/null

echo "== 1. A bodice: polygon, curved armhole, notches, shoulder width 13 cm"
"$R3D" run "$OUT" /dev/stdin <<'JSON' | grep -E '"(created|distance_mm)"' | head -4
[{"type": "panel.create_polygon", "name": "Bodice",
  "points": [[0, 0], [240, 0], [250, 300], [200, 420], [80, 440], [0, 430]]},
 {"type": "edge.set_curve", "edge": "$0.created[9]",
  "curve": {"kind": "cubic", "c1": [245, 360], "c2": [215, 400]}},
 {"type": "notch.add", "edge": "$0.created[8]", "offset_mm": 120},
 {"type": "notch.add", "edge": "$0.created[9]", "offset_mm": 60},
 {"type": "point.set_distance", "from": "$0.created[5]", "point": "$0.created[4]",
  "distance_mm": 130},
 {"type": "measure.distance", "a": "$0.created[5]", "b": "$0.created[4]"}]
JSON

echo "== 2. A skirt gathered 2:1 into a waistband"
"$R3D" run "$OUT" /dev/stdin <<'JSON' >/dev/null
[{"type": "panel.create_rect", "name": "Band", "origin": [400, 700], "width": 760, "height": 60},
 {"type": "panel.create_rect", "name": "Skirt", "origin": [400, 0], "width": 1520, "height": 500},
 {"type": "seam.add", "a": {"edge": "$0.created[8]"}, "b": {"edge": "$0.created[6]"}},
 {"type": "seam.add", "a": {"edge": "$1.created[8]"}, "b": {"edge": "$1.created[6]"}},
 {"type": "seam.add", "a": {"edge": "$0.created[5]"}, "b": {"edge": "$1.created[7]"}},
 {"type": "seam.set_gathered", "seam": "$4.created[0]", "gathered": true},
 {"type": "panel.place", "panel": "$0.created[0]", "around": "body", "angle_deg": 0, "top_cm": 112},
 {"type": "panel.place", "panel": "$1.created[0]", "around": "body", "angle_deg": 0, "top_cm": 106}]
JSON
"$R3D" query "$OUT" '{"type": "seam.list"}' | grep -E '"(gathered|gathering_ratio)"'

echo "== 3. Drape it (4 s) and render"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../gathered.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern.png" >/dev/null
ls -1 "$(dirname "$OUT")"/*.png
