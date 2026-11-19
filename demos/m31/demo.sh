#!/usr/bin/env bash
# M31 demo: imported avatars. We make a "scan" of our own: the built-in
# CC0 body exported to OBJ, rewritten in centimetres with Z up (as many
# modelling tools write it) and its soles cut open (as a scan often is).
# A script (or an agent) imports it: units, up axis and facing are
# detected, the check reports the holes, the arrangement points are
# guessed inside the body and the measurements taken on it. A wrong turn
# is flagged by the check and corrected. A T-shirt sized from those
# measurements is placed around the body, draped and rendered.
# In the app: Body › Import body…, then Check body… and Show arrangement
# points.
# Run from the repository root: ./demos/m31/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/My scan.r3d"

echo "== 1. A body file of our own: the built-in body, in cm, Z up, soles open"
SRC="$DIR/Source.r3d"
"$R3D" new "$SRC" >/dev/null
"$R3D" run "$SRC" - >/dev/null <<'JSON'
[{"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
  "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}]
JSON
"$R3D" export "$SRC" --format obj --avatar --out exports/body.obj >/dev/null
# Metres, Y up, facing +Z  ->  centimetres, Z up, facing -Y; faces whose
# three corners are within 1.5 cm of the floor are dropped (the soles).
awk '
  /^v / { n++; h[n] = $3; printf "v %.4f %.4f %.4f\n", $2 * 100, -$4 * 100, $3 * 100; next }
  /^f / {
    low = 1
    for (i = 2; i <= NF; i++) { split($i, k, "/"); if (h[k[1]] >= 0.015) low = 0 }
    if (low) next
  }
  /^(vn|vt) / { next }
  { print }
' "$SRC/exports/body.obj" > "$DIR/scan-cm-zup.obj"
echo "   $(grep -c '^v ' "$DIR/scan-cm-zup.obj") vertices, $(grep -c '^f ' "$DIR/scan-cm-zup.obj") faces"

echo "== 2. Imported: what was detected, and the check"
"$R3D" new "$OUT" >/dev/null
"$R3D" avatar import "$OUT" "$DIR/scan-cm-zup.obj" > "$DIR/import.json"
tr -d '\n ' < "$DIR/import.json" | grep -oE '"(scale|up|turn_deg|height_m|triangles|ready)":[^,]*' \
  | sed 's/^/   /'
echo "   items:"
tr -d '\n ' < "$DIR/import.json" | grep -oE '"code":"[a-z_.]+","severity":"[a-z]+"' \
  | sed -E 's/"code":"([^"]*)","severity":"([^"]*)"/     \2 \1/'
echo "   measurements taken on it:"
tr -d '\n ' < "$DIR/import.json" | grep -oE '"body":\{[^}]*\}' | sed 's/^/     /'
echo "   arrangement points (distance to the surface, mm; negative inside):"
tr -d '\n ' < "$DIR/import.json" \
  | grep -oE '"distance_mm":-?[0-9.]+,"point":"[a-z_]+"' \
  | sed -E 's/"distance_mm":(-?[0-9.]+),"point":"([a-z_]+)"/     \2 \1/'

echo "== 3. A wrong turn is flagged, then corrected"
"$R3D" run "$OUT" - >/dev/null <<'JSON'
[{"type": "avatar.orient", "turn_deg": 180}]
JSON
"$R3D" avatar check "$OUT" | grep -oE '"code": "orientation[a-z_.]*"' | sed 's/^/   turned 180: /'
"$R3D" run "$OUT" - >/dev/null <<'JSON'
[{"type": "avatar.orient", "turn_deg": 0}]
JSON
"$R3D" avatar check "$OUT" | grep -cE '"code": "orientation' | sed 's/^/   back to 0: orientation warnings: /' || true

echo "== 4. A T-shirt sized from those measurements, placed around the body and draped"
"$R3D" run "$OUT" - >/dev/null <<'JSON'
[{"type": "template.apply", "template": "t_shirt"},
 {"type": "project.rename", "name": "T-shirt on a scan"}]
JSON
"$R3D" simulate "$OUT" --frames 240 \
  | tr -d '\n ' | grep -oE '"(frames|max_seam_gap_mm|settled)":[^,}]*|"body_warnings":\[[^]]*\]' \
  | sed 's/^/   /'

echo "== 5. Rendered to PNG: the 3D view and a photo"
"$R3D" render "$OUT" --view 3d --out "$DIR/scan-3d.png" >/dev/null
"$R3D" render "$OUT" --view photo --quality quick --camera three-quarter \
  --out "$DIR/scan-photo.png" | grep -E '"(samples|seconds)"' | tr -d ' ,' | paste -sd' ' \
  | sed 's/^/   photo: /'
echo "   the body file in the project: $(ls "$OUT/avatars")"
ls -1 "$DIR"/scan-*.png
