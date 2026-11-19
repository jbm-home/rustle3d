#!/usr/bin/env bash
# M21 demo: textures and prints. A script (or an agent) makes a shirt in
# a navy and off-white stripe, 2 cm per repeat: the stripes run along the
# grainline of every piece (fronts, back, sleeves, cuffs, collar). It
# drapes it, renders the pattern, the 3D view and a photo to PNG, exports
# glTF (the print embedded) and OBJ (with its .mtl and image), then
# switches to a print image of its own (a check drawn here as a PNG),
# 30 cm wide and turned 45° on one front. In the app: the garment's card
# in the inspector (nothing selected), Print.
# Run from the repository root: ./demos/m21/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/Striped shirt.r3d"

"$R3D" new "$OUT" >/dev/null
cat > "$DIR/shirt.json" <<'JSON'
[
  {"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
   "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}},
  {"type": "template.apply", "template": "shirt"},
  {"type": "garment.set_color", "color": "#1F3A70"},
  {"type": "project.rename", "name": "Striped shirt"}
]
JSON
"$R3D" run "$OUT" "$DIR/shirt.json" >/dev/null

echo "== 1. Stripes along the grain, 2 cm per repeat"
"$R3D" texture "$OUT" stripes --width-mm 20 | tr -d '\n ' \
  | grep -oE '"print":\{[^}]*\}[^}]*\}' | sed 's/^/   /'
echo
echo "   each piece's grainline (degrees from the board's x axis):"
PANELS=$("$R3D" query "$OUT" '{"type": "project.summary"}' \
  | grep -oE '"id": "panel:[0-9]+"' | cut -d'"' -f4)
for id in $PANELS; do
  "$R3D" query "$OUT" "{\"type\": \"panel.get\", \"panel\": \"$id\"}" > "$DIR/panel.json"
  name=$(grep -m1 -oE '^  "name": "[^"]*"' "$DIR/panel.json" | cut -d'"' -f4)
  grain=$(grep -m1 -oE '"grain_angle_deg": [0-9.-]+' "$DIR/panel.json" | cut -d' ' -f2)
  printf "     %-16s %s\n" "$name" "$grain"
done

echo "== 2. Drape it (5 s of cloth)"
"$R3D" simulate "$OUT" --frames 300 | grep -E '"(frames|max_seam_gap_mm|settled)"' | tr -d ' ,' | paste -sd' '

echo "== 3. Rendered to PNG: the pattern, the 3D view, a photo"
"$R3D" render "$OUT" --view board --out "$DIR/shirt-board.png" >/dev/null
"$R3D" render "$OUT" --view 3d --out "$DIR/shirt-3d.png" >/dev/null
"$R3D" render "$OUT" --view photo --quality quick --camera front --out "$DIR/shirt-photo.png" \
  | grep -E '"(samples|seconds)"' | tr -d ' ,' | paste -sd' ' | sed 's/^/   photo: /'

echo "== 4. Exported: glTF with the print inside, OBJ with its material and image"
"$R3D" export "$OUT" --format glb --avatar | grep -E '"path"' | sed 's/^ */   /'
"$R3D" export "$OUT" --format obj | grep -E '"(path|mtl)"|png' | sed 's/^ */   /'
if command -v node >/dev/null && [ -d target/gltf-validator/node_modules/gltf-validator ]; then
  cp scripts/validate-gltf.mjs target/gltf-validator/
  node target/gltf-validator/validate-gltf.mjs "$OUT/exports/Striped shirt.glb" | sed 's/^/   /'
fi

echo "== 5. A print image of our own: a check, 30 cm wide, turned 45° on a front"
PYTHON=python3
"$PYTHON" -c '' 2>/dev/null || PYTHON=python
"$PYTHON" - "$DIR/check.png" <<'PY'
import struct, sys, zlib
w = h = 64
rows = b"".join(
    b"\0" + b"".join(
        bytes((150, 30, 40) if (x // 16 + y // 16) % 2 else (240, 225, 200))
        for x in range(w))
    for y in range(h))
def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
png = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))
open(sys.argv[1], "wb").write(png)
PY
"$R3D" texture "$OUT" "$DIR/check.png" --width-mm 300 | tr -d '\n ' \
  | grep -oE '"print":\{[^}]*\}[^}]*\}' | sed 's/^/   /'
echo
FRONT=$(echo "$PANELS" | head -1)
"$R3D" run "$OUT" - >/dev/null <<JSON
[{"type": "panel.set_print_placement", "panel": "$FRONT", "angle_deg": 45}]
JSON
"$R3D" render "$OUT" --view 3d --out "$DIR/shirt-check-3d.png" >/dev/null
echo "   in the project's textures/: $(ls "$OUT/textures")"

ls -1 "$DIR"/shirt-*.png
