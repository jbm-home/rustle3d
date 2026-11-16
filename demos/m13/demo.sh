#!/usr/bin/env bash
# M13 demo: a pattern drawn elsewhere comes in, is edited, sewn, draped
# and goes out again. The skirt is a DXF-AAMA file in inches (two blocks
# drawn at the origin, a sewing line inside the cutting line, notches, a
# dart); the bodice an SVG from a vector editor. Both are reference files
# of the tests (crates/r3d-commands/tests/data). In the app: Import in the
# top bar, Export → "Pattern, DXF-AAMA". Run from the repository root.
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DATA=crates/r3d-commands/tests/data
TMP="$(mktemp -d)"
OUT="$TMP/Imported.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" /dev/stdin >/dev/null <<'JSON'
[{"type": "avatar.set", "preset": "Women M",
  "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}]
JSON

echo "== 1. Import a skirt drawn elsewhere: DXF-AAMA in inches (r3d import copies it into imports/)"
"$R3D" import "$OUT" "$DATA/skirt-aama-inches.dxf"
"$R3D" query "$OUT" '{"type": "panel.list"}' | grep -E '"(name|area_mm2)"'

echo "== 2. Sew and edit, as an agent would: side seams, around the body, 5 cm longer"
"$R3D" run "$OUT" /dev/stdin >/dev/null <<'JSON'
[{"type": "panel.list"},
 {"type": "panel.get", "panel": "$0[0].id"},
 {"type": "panel.get", "panel": "$0[1].id"},
 {"type": "seam.add", "a": {"edge": "$1.edges[1].id"}, "b": {"edge": "$2.edges[4].id"}},
 {"type": "seam.add", "a": {"edge": "$2.edges[1].id"}, "b": {"edge": "$1.edges[3].id"}},
 {"type": "garment.arrange"},
 {"type": "edge.lengthen", "edge": "$1.edges[0].id", "mm": 50}]
JSON
"$R3D" query "$OUT" '{"type": "seam.list"}' | grep -c '"id"' | sed 's/^/seams: /'
"$R3D" query "$OUT" '{"type": "panel.list"}' | grep -E '"(name|area_mm2)"' 

echo "== 3. Drape it (4 s)"
"$R3D" simulate "$OUT" --frames 240 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'
"$R3D" render "$OUT" --view 3d --out "$TMP/skirt.png" >/dev/null

echo "== 4. Export it back (DXF-AAMA, SVG, A4 tiles) and read each file again in a fresh project"
for f in dxf svg pdf; do
  path=$("$R3D" export "$OUT" --format $f | grep '"path"' | sed 's/.*: "\(.*\)".*/\1/')
  "$R3D" new "$TMP/Back-$f.r3d" >/dev/null
  printf '%s: ' "$f"
  result=$("$R3D" import "$TMP/Back-$f.r3d" "$OUT/$path")
  echo "$(grep -o '"panel:' <<<"$result" | wc -l | tr -d ' ') pieces, scale $(grep -o '"scale": "[a-z]*"' <<<"$result" | cut -d'"' -f4)"
done

echo "== 5. An SVG bodice from a vector editor (mm, transforms, dashed sewing line)"
"$R3D" import "$OUT" "$DATA/bodice-inkscape.svg"
"$R3D" query "$OUT" '{"type": "panel.list"}' | grep -E '"name"'
"$R3D" render "$OUT" --view board --out "$TMP/pattern.png" >/dev/null
ls -1 "$TMP"/*.png
