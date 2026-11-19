#!/usr/bin/env bash
# M20 demo: trims, buttons and zippers. A script (or an agent) drafts a
# shirt that buttons down the front and at the cuffs, and trousers with a
# fly zipper, on a men's body; drapes both closed and renders them (the
# pattern with its buttons, buttonholes and zipper; the garment in 3D
# with its buttons and the zipper's band); then unbuttons the shirt's two
# top buttons and opens the fly, drapes again and renders the result; and
# writes the shirt's tech pack, whose bill of materials lists the
# buttons. In the app: the rail's Trims menu (Button, Zipper), and a
# piece's card (Buttoned, Unbutton all, a zipper's Open slider).
# Run from the repository root: ./demos/m20/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
BODY='{"type": "avatar.set", "preset": "Men M", "body": {"masculinity": 1,
  "height_cm": 178, "bust_cm": 100, "waist_cm": 86, "hips_cm": 100}}'

for garment in shirt trousers; do
  "$R3D" new "$DIR/$garment.r3d" >/dev/null
  "$R3D" run "$DIR/$garment.r3d" - >/dev/null <<JSON
[$BODY, {"type": "template.apply", "template": "$garment"}]
JSON
done

echo "== 1. The shirt's buttons, the trousers' fly zipper (trim.list)"
"$R3D" query "$DIR/shirt.r3d" '{"type": "trim.list"}' > "$DIR/shirt-trims.json"
echo "   shirt: $(grep -c '"id": "button' "$DIR/shirt-trims.json") buttons of" \
  "$(grep -m1 -oE '"diameter_mm": [0-9.]+' "$DIR/shirt-trims.json" | cut -d' ' -f2) mm," \
  "$(grep -c '"panel_name": "Cuff' "$DIR/shirt-trims.json") on the cuffs"
"$R3D" query "$DIR/trousers.r3d" '{"type": "trim.list"}' > "$DIR/trousers-trims.json"
echo "   trousers: a zipper $(grep -oE '"length_mm": [0-9.]+' "$DIR/trousers-trims.json" | cut -d' ' -f2) mm long," \
  "closed $(grep -oE '"closed": [0-9.]+' "$DIR/trousers-trims.json" | cut -d' ' -f2)"

echo "== 2. Draped closed (4 s of cloth): buttons and zipper hold"
for garment in shirt trousers; do
  "$R3D" simulate "$DIR/$garment.r3d" --frames 240 \
    | grep -E '"(closures|max_closure_gap_mm|max_seam_gap_mm|top_m|bottom_m)"' \
    | tr -d ' ,' | paste -sd' ' | sed "s/^/   $garment: /"
  "$R3D" render "$DIR/$garment.r3d" --view board --out "$DIR/$garment-board.png" >/dev/null
  "$R3D" render "$DIR/$garment.r3d" --view 3d --out "$DIR/$garment-3d.png" >/dev/null
done

echo "== 3. Two top buttons undone, the fly opened, draped again"
TOP=$(grep -B3 '"panel_name": "Front' "$DIR/shirt-trims.json" | grep -oE '"id": "button:[0-9]+"' \
  | cut -d'"' -f4 | head -2 | sed 's/.*/"&"/' | paste -sd,)
"$R3D" run "$DIR/shirt.r3d" - >/dev/null <<JSON
[{"type": "button.set_closed", "buttons": [$TOP], "closed": false}]
JSON
ZIP=$(grep -oE '"id": "zipper:[0-9]+"' "$DIR/trousers-trims.json" | cut -d'"' -f4)
"$R3D" run "$DIR/trousers.r3d" - >/dev/null <<JSON
[{"type": "zipper.set_open", "zipper": "$ZIP", "open_mm": 180}]
JSON
for garment in shirt trousers; do
  "$R3D" simulate "$DIR/$garment.r3d" --frames 240 \
    | grep -E '"(closures|max_closure_gap_mm)"' | tr -d ' ,' | paste -sd' ' \
    | sed "s/^/   $garment: /"
  "$R3D" render "$DIR/$garment.r3d" --view 3d --out "$DIR/$garment-open-3d.png" >/dev/null
done

echo "== 4. The shirt's tech pack: the buttons in the bill of materials"
"$R3D" techpack "$DIR/shirt.r3d" | grep -E '"path"' | sed 's/^ */   /'
if grep -aq "Button" "$DIR/shirt.r3d/exports/"*techpack.pdf; then
  echo "   the bill of materials lists the buttons"
fi

ls -1 "$DIR"/*.png
