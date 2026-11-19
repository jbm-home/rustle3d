#!/usr/bin/env bash
# M10 demo: a starter garment drafted to the body. An agent (or a script)
# lists the templates with their sizes for a men's L body, drafts a T-shirt
# with longer sleeves, drapes it headless, and renders the pattern and the
# garment; the same flow is the app's start screen (New T-shirt).
# Run from the repository root: ./demos/m10/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(demo_out m10)/Shirt.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m10/body.json >/dev/null

echo "== 1. The templates and the sizes they read, filled in from the body"
"$R3D" query "$OUT" '{"type": "template.list"}' | grep -E '"(template|chest_cm|waist_cm|length_cm|sleeve_cm)"'

echo "== 2. A T-shirt with 25 cm sleeves: pieces, seams (lengths matched)"
"$R3D" run "$OUT" demos/m10/shirt.json | grep -E '"mismatch_mm"' | sort | uniq -c

echo "== 3. Drape it headless (5 s), then render"
"$R3D" simulate "$OUT" --frames 300 | grep -E '"(top_m|bottom_m|max_seam_gap_mm|ms_per_frame)"'
"$R3D" render "$OUT" --view 3d --out "$OUT/../shirt.png" >/dev/null
"$R3D" render "$OUT" --view board --out "$OUT/../pattern.png" >/dev/null
ls -1 "$(dirname "$OUT")"/*.png
