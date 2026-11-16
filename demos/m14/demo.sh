#!/usr/bin/env bash
# M14 demo: a path-traced studio photo of the draped garment. A T-shirt in
# a teal jersey is drafted to the body and draped headless, then
# photographed in the three studio setups, in Good quality from the front,
# and by an agent over MCP (render_photo). In the app: the camera button
# over the 3D view. Run from the repository root: ./demos/m14/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/Shirt.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m14/shirt.json >/dev/null

echo "== 1. Drape the T-shirt (5 s)"
"$R3D" simulate "$OUT" --frames 300 | grep -E '"(max_seam_gap_mm|settled)"'

echo "== 2. Quick photos in each studio setup (seconds each)"
for light in studio soft dramatic; do
  printf '%-9s' "$light"
  "$R3D" render "$OUT" --view photo --quality quick --lighting "$light" \
    --size 800x600 --out "$DIR/quick-$light.png" | grep '"seconds"'
done

echo "== 3. A Good photo from the front, the whole body (128 samples per pixel)"
"$R3D" render "$OUT" --view photo --quality good --camera front --frame body \
  --size 1080x1350 --out "$DIR/good-front.png" | grep -E '"(samples|seconds)"'

echo "== 4. An agent asks for a photo over MCP, then makes a mistake it can read"
"$R3D" mcp --project "$OUT" < demos/m14/mcp-session.jsonl \
  | sed -nE 's/.*"id":([0-9]+).*"text":"(.*)"\}\],.*/  #\1 \2/p' \
  | sed 's/\\"/"/g' | cut -c1-160

ls -1 "$DIR"/*.png
