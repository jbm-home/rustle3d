#!/usr/bin/env bash
# M23 demo: animated avatars. A script (or an agent) dresses the women's
# M body in the dress template, then drapes it headless while the avatar
# walks on the spot: the dress closes on the still body for 1.5 s, then
# the avatar eases into its walk and the skirt follows. The same with a
# skirt turning half round and a T-shirt raising its arms. Each drape is
# cached with its motion, so the 3D render and the GLB export show the
# body and the garment as they are at the last frame. In the app: the
# walking figure next to Simulate.
# Run from the repository root: ./demos/m23/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"

step=1
for case in "dress walk 225" "a_line_skirt turn 240" "t_shirt raise_arms 200"; do
  set -- $case
  template=$1 motion=$2 frames=$3
  OUT="$DIR/$template.r3d"
  "$R3D" new "$OUT" >/dev/null
  cat > "$DIR/$template.json" <<JSON
[
  {"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
   "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}},
  {"type": "template.apply", "template": "$template"}
]
JSON
  "$R3D" run "$OUT" "$DIR/$template.json" >/dev/null
  echo "== $step. $template, the avatar does '$motion' ($frames frames)"
  "$R3D" simulate "$OUT" --frames "$frames" --motion "$motion" \
    | grep -E '"(motion|motion_s|max_seam_gap_mm|top_m|bottom_m)"' | tr -d ' ,' | paste -sd' '
  "$R3D" render "$OUT" --view 3d --out "$DIR/$template-$motion.png" >/dev/null
  step=$((step + 1))
done

echo "== $step. The walking dress, body and garment as at the last frame, as GLB"
"$R3D" export "$DIR/dress.r3d" --format glb --avatar | grep '"path"'
ls -1 "$DIR"/*.png
