#!/usr/bin/env bash
# M24 demo: fit maps and the virtual tape measure. A script (or an agent)
# dresses the women's M body in a T-shirt long enough to cover the hips,
# drapes it headless, then reads how it fits: the 3D view coloured by
# strain (how much the cloth is stretched beyond its pattern) and by gap
# (how far it stands off the body), both on the viridis scale (dark
# loose, yellow tight; the bar at the right edge); the tightest piece;
# the tight spots; the ease at the chest, waist and hips. In the app:
# the gauge button over the 3D view, and the Tape measure tool (M).
# Run from the repository root: ./demos/m24/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/Tee.r3d"

"$R3D" new "$OUT" >/dev/null
cat > "$DIR/tee.json" <<'JSON'
[
  {"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
   "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}},
  {"type": "template.apply", "template": "t_shirt", "sizes": {"length_cm": 68}}
]
JSON
"$R3D" run "$OUT" "$DIR/tee.json" >/dev/null

echo "== 1. Drape the T-shirt (5 s of cloth)"
"$R3D" simulate "$OUT" --frames 300 | grep -E '"(frames|max_seam_gap_mm|settled)"' | tr -d ' ,' | paste -sd' ' -

echo "== 2. Fit maps rendered to PNG: strain and gap"
for map in strain gap; do
  "$R3D" render "$OUT" --view 3d --fit-map "$map" --out "$DIR/tee-$map.png" \
    | grep -E '"(piece|value|unit)"' | sed 's/^ *//; s/,$//' | paste -sd' ' - \
    | sed "s/^/   $map, the tightest: /"
done

echo "== 3. Strain per piece (max and mean %, share of the area above 5 %)"
"$R3D" fit "$OUT" > "$DIR/fit.json"
tr -d '",' < "$DIR/fit.json" | awk '
  /^  pieces:/ { p = 1 } /^  threshold_pct:/ { p = 0 }
  p && /max_strain_pct:/ { mx = $2 } p && /mean_strain_pct:/ { mn = $2 }
  p && /name:/ { $1 = ""; name = $0 } p && /tight_area_pct:/ { ta = $2 }
  p && /touching_pct:/ { printf "   %-14s max %5s  mean %4s  tight %5s %%\n", name, mx, mn, ta }'

echo "== 4. The tight spots, strongest first (piece, peak %, area cm2, height cm)"
tr -d '",' < "$DIR/fit.json" | awk '
  /^  tight_spots:/ { t = 1 }
  t && /area_cm2:/ { a = $2 } t && /height_cm:/ { h = $2 } t && /name:/ { $1 = ""; n = $0 }
  t && /strain_pct:/ { if (k++ < 5) printf "   %-14s %5s %%  %6s cm2  at %s cm\n", n, $2, a, h }'

echo "== 5. Ease (garment minus body circumference), from the virtual tape"
tr -d '",' < "$DIR/fit.json" | awk '
  /^  ease:/ { e = 1 } /^  pieces:/ { e = 0 }
  e && /at:/ { at = $2 } e && /body_cm:/ { b = $2 } e && /ease_cm:/ { ea = $2 }
  e && /garment_cm:/ { g = $2 }
  e && /height_cm:/ { printf "   %-6s at %5s cm: garment %6s cm  body %6s cm  ease %5s cm\n", at, $2, g, b, ea }'

echo "== 6. The tape at a height an agent chooses (110 cm)"
"$R3D" run "$OUT" - <<<'[{"type": "fit.tape", "height_cm": 110}]' \
  | tr -d '\n ' | grep -oE '"(garment_cm|body_cm|ease_cm)":[0-9.-]+' | paste -sd' ' - | sed 's/^/   /'

ls -1 "$DIR"/*.png
