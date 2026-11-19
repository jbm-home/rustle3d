#!/usr/bin/env bash
# M12 demo: a piece drawn with curves, and a skirt cut along a princess
# line. An agent (or a script) draws a curved yoke with
# `panel.create_path`, then cuts the A-line skirt's front along a curved
# line with `panel.cut`: two pieces, sewn together along the cut, that
# drape like the whole front. In the app: Curve (C: click for a corner,
# drag for a smooth point), Cut (K: click across a piece, Enter), and the
# Edit curves group (Bend B, Straighten L, Add point A). Run from the
# repository root.
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(demo_out m12)/Princess.r3d"

"$R3D" new "$OUT" >/dev/null
# A skirt hangs from the hips: a women's body (on the men's preset the
# solver's grip at the waist is too weak for heavier skirts, ROADMAP).
"$R3D" run "$OUT" /dev/stdin >/dev/null <<'JSON'
[{"type": "avatar.set", "preset": "Women M",
  "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}]
JSON

echo "== 1. The A-line skirt, its front cut along a curved princess line"
"$R3D" run "$OUT" /dev/stdin <<'JSON' | grep -E '"name"' | sort -u
[{"type": "template.apply", "template": "a_line_skirt", "sizes": {"length_cm": 60}},
 {"type": "panel.cut", "start": [220, 650], "name": "Front left",
  "segments": [{"kind": "cubic", "c1": [220, 400], "c2": [180, 200], "to": [180, -50]}]},
 {"type": "panel.cut", "start": [500, 650], "name": "Front right",
  "segments": [{"kind": "cubic", "c1": [500, 400], "c2": [540, 200], "to": [540, -50]}]},
 {"type": "project.summary"}]
JSON
"$R3D" query "$OUT" '{"type": "seam.list"}' | grep -c '"id"' | sed 's/^/seams: /'

echo "== 2. A curved yoke drawn in one go (straight sides, curved top and bottom)"
"$R3D" run "$OUT" /dev/stdin <<'JSON' | grep -E '"(area_mm2|perimeter_mm)"'
[{"type": "panel.create_path", "name": "Yoke", "start": [0, 900], "segments": [
   {"kind": "cubic", "c1": [150, 860], "c2": [350, 860], "to": [500, 900]},
   {"kind": "line", "to": [500, 1000]},
   {"kind": "cubic", "c1": [350, 960], "c2": [150, 960], "to": [0, 1000]},
   {"kind": "line", "to": [0, 900]}]},
 {"type": "panel.get", "panel": "$0.created[0]"}]
JSON

echo "== 3. Drape it (4 s) and render"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../princess.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern.png" >/dev/null
ls -1 "$(dirname "$OUT")"/*.png
