#!/usr/bin/env bash
# M18 demo: shirt and trousers templates. An agent (or a script) drafts a
# shirt and trousers for a men's L body, drapes each headless and renders
# it; in the app, the start screen offers both (New → Shirt, Trousers).
# Run from the repository root: ./demos/m18/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(demo_out m18)"

for garment in shirt trousers; do
  OUT="$DIR/$garment.r3d"
  "$R3D" new "$OUT" >/dev/null
  "$R3D" run "$OUT" demos/m10/body.json >/dev/null
  echo "== $garment: sizes from the body, pieces, seams (lengths matched)"
  "$R3D" query "$OUT" '{"type": "template.list"}' \
    | grep -B5 "\"template\": \"$garment\"" | grep -E "_cm"
  echo "[{\"type\": \"template.apply\", \"template\": \"$garment\"}, {\"type\": \"seam.list\"}]" \
    > "$DIR/$garment.json"
  "$R3D" run "$OUT" "$DIR/$garment.json" | grep -E '"mismatch_mm"' | sort | uniq -c
  echo "== $garment: drape it headless (3 s), then render"
  "$R3D" simulate "$OUT" --frames 180 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
  "$R3D" render "$OUT" --view photo --quality quick --camera front --frame body \
    --out "$DIR/$garment.png" >/dev/null
done
ls -1 "$DIR"/*.png
