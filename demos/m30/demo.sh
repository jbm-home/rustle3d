#!/usr/bin/env bash
# M30 demo: production. A shirt graded 36 to 52 (drawn in 40), with a
# 1 cm seam allowance, in a cotton sold 150 cm wide at 12.50 a metre:
# its indicators per piece, then sizes 38, 40 and 42 nested on the fabric
# (a marker): length, efficiency, yardage and cost, written as SVG and PDF
# at 1:1. Then its notes, a drape, and its tech pack (PDF): front and back
# views, colourways, bill of materials, measurements, pattern, sewing
# notes. The projector is a view of the app, see step 4.
# Run from the repository root: ./demos/m30/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/Shirt.r3d"

"$R3D" new "$OUT" >/dev/null
cat > "$DIR/shirt.json" <<'JSON'
[
  {"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"},
  {"type": "template.apply", "template": "shirt"},
  {"type": "garment.set_fabric_details", "weight_gsm": 130, "width_mm": 1500, "price_per_m": 12.5},
  {"type": "garment.set_color", "color": "#7A9CC6"},
  {"type": "garment.set_notes", "notes": "Interface the collar and the cuffs.\nTopstitch the collar 6 mm from the edge.\nPress every seam open, then overlock."}
]
JSON
"$R3D" run "$OUT" "$DIR/shirt.json" >/dev/null

# A 1 cm seam allowance on every piece drawn (mirror copies follow).
"$R3D" query "$OUT" '{"type": "panel.list"}' | grep -o '"panel:[0-9]*"' | sort -u \
  | while read -r p; do
      echo "[{\"type\": \"panel.set_seam_allowance\", \"panel\": $p, \"mm\": 10}]" > "$DIR/sa.json"
      "$R3D" run "$OUT" "$DIR/sa.json" >/dev/null 2>&1 || true   # linked copies refuse
    done

echo "== 1. Indicators of size 40: each piece's cut area (mm2) and mass (g), the totals"
"$R3D" query "$OUT" '{"type": "production.indicators", "size": "40"}' \
  | tr -d '",' | awk '
      /"?totals/ { t = 1 }
      /cut_area_mm2:/ { c = $2 }
      /mass_g:/ { m = $2; if (t) { printf "  %-14s %12s mm2 %8s g\n", "Total", c, m; t = 0 } }
      /name:/ { $1 = ""; if (c != "") printf "  %-14s %12s mm2 %8s g\n", $0, c, m; c = "" }'

echo "== 2. Sizes 38, 40 and 42 nested on 150 cm"
"$R3D" marker "$OUT" --sizes 38,40,42 --width-cm 150 --format svg \
  | grep -E '"(length_mm|efficiency_pct|yardage_m|price_per_m|cost|pieces|path)"' | tr -d ' ,' | paste -sd' ' -
"$R3D" marker "$OUT" --sizes 38,40,42 --width-cm 150 --format pdf | grep '"path"'

echo "== 3. The tech pack: drape (2.5 s of cloth), then front and back views and the rest"
"$R3D" simulate "$OUT" --frames 150 | grep -E '"(frames|max_seam_gap_mm)"' | tr -d ' ,' | paste -sd' ' -
"$R3D" techpack "$OUT" | grep -E '"(yardage_m|cost|images|path)"' | tr -d ' ,' | paste -sd' ' -
ls -1 "$OUT"/exports/

cat <<'TEXT'
== 4. The back piece projected at 1:1 (in the app)
   Open the project in the app (start screen: Open…, the folder
   printed below), select the Back on the board, then Export ->
   Projector. Move the window to the projector and maximise it; press C,
   measure the projected square on the table and type its sides; drag or
   use the arrows to bring the back under the lamp; M mirrors it.
TEXT
echo "Project: $OUT"
