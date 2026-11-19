#!/usr/bin/env bash
# M19 demo: several garments in layers. An agent (or a script) drafts
# trousers, then a shirt (a garment of its own, worn over them), gives the
# shirt a colour, drapes both headless and renders a photo, each garment
# in its fabric and colour. A shirt tucked in (garment.set_layer to 0) is
# not draped well yet (docs/specs/17).
# Run from the repository root: ./demos/m19/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
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

echo "== 2. Drape both (4 s), then a photo"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
"$R3D" render "$OUT" --view photo --quality quick --camera front --frame body \
  --out "$DIR/outfit.png" >/dev/null
ls -1 "$DIR"/*.png
