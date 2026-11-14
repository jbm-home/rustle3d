#!/usr/bin/env bash
# M2 demo: an A-line skirt pattern (curved waist, seam allowances, notches,
# a centre line, a linked mirror back piece) built by a CLI script, then an
# AI-agent-style MCP session on the same project, then 1:1 exports.
# Run from the repository root: ./demos/m2/demo.sh
set -euo pipefail

cargo build --quiet -p r3d-cli
R3D=./target/debug/r3d
OUT="$(mktemp -d)/Skirt.r3d"

echo "== 1. Build the pattern with a script"
"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m2/skirt.json | grep -E '"(name|area_mm2|length_mm|is_simple)"' | head -n 12

echo "== 2. An agent session over MCP (stdio): list, edit, a refused delete,"
echo "      an export, and a write outside the project that is refused"
"$R3D" mcp --project "$OUT" < demos/m2/mcp-session.jsonl \
  | sed -nE 's/.*"id":([0-9]+).*"text":"(.*)"\}\],.*/  #\1 \2/p' \
  | sed 's/\\"/"/g' | cut -c1-150

echo "== 3. Exports (1:1): SVG, tiled A4 PDF, one-page PDF"
"$R3D" export "$OUT" --format svg
"$R3D" export "$OUT" --format pdf --paper single --out exports/plotter.pdf
ls -1 "$OUT/exports"

echo
echo "Open $OUT/exports/ and print the A4 PDF at 100 %: the test square must measure 100 mm."
echo "To drive this project from Claude Code:"
echo "  claude mcp add rustle3d -- $(pwd)/target/debug/r3d mcp --project $OUT"
