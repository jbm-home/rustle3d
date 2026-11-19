#!/usr/bin/env bash
# M19 demo: several garments in layers. An agent (or a script) drafts
# trousers, then a shirt (a garment of its own, worn over them), gives the
# shirt a colour, drapes both headless and renders a photo, each garment
# in its fabric and colour; then wears the shirt under the trousers
# (garment.set_layer to 0) and drapes again: the shirt is tucked in, the
# garments lying as close as their fabrics are thick (thin contacts
# between garments, docs/physics.md "Several garments").
# Run from the repository root: ./demos/m19/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(demo_out m19)"
OUT="$DIR/Outfit.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m10/body.json >/dev/null
cat > "$DIR/outfit.json" <<'JSON'
[
  {"type": "template.apply", "template": "trousers"},
  {"type": "template.apply", "template": "shirt"}
]
JSON
"$R3D" run "$OUT" "$DIR/outfit.json" >/dev/null

echo "== 1. The garments, next to the body first"
"$R3D" query "$OUT" '{"type": "project.summary"}' \
  | grep -E '"(id|name|fabric|layer)": "?(garment|Trousers|Shirt|denim|cotton|[0-9])' | head -8
SHIRT=$("$R3D" query "$OUT" '{"type": "project.summary"}' \
  | grep -B3 '"name": "Shirt"' | grep '"id"' | sed 's/.*"id": "\(.*\)".*/\1/')
echo "[{\"type\": \"garment.set_color\", \"garment\": \"$SHIRT\", \"color\": \"#d8e4f0\"}]" \
  > "$DIR/color.json"
"$R3D" run "$OUT" "$DIR/color.json" >/dev/null

echo "== 2. The shirt worn over the trousers: drape both (4 s), then a photo"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
"$R3D" render "$OUT" --view photo --quality quick --camera front --frame body \
  --out "$DIR/outfit.png" >/dev/null

echo "== 3. The shirt worn under them (layer 0): tucked in, front and back"
echo "[{\"type\": \"garment.set_layer\", \"garment\": \"$SHIRT\", \"layer\": 0}]" \
  > "$DIR/tuck.json"
"$R3D" run "$OUT" "$DIR/tuck.json" >/dev/null
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
for camera in front back; do
  "$R3D" render "$OUT" --view photo --quality quick --camera "$camera" --frame body \
    --out "$DIR/tucked-$camera.png" >/dev/null
done
ls -1 "$DIR"/*.png
