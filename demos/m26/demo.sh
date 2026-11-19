#!/usr/bin/env bash
# M26 demo: history, semantic copy and paste, feedback on long steps.
# A script draws two pieces, sews them and moves them, lists the history,
# jumps back ten steps and forward again; copies the two sewn pieces into
# a new project (their seam comes along); then adds the shirt template to
# a project graded 36-52 and shows its sizes being drafted (progress on
# stderr in a terminal; MCP clients get progress notifications).
# Run from the repository root: ./demos/m26/demo.sh
set -euo pipefail
. "$(dirname "$0")/../out.sh"

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(demo_out m26)"
A="$DIR/Skirt.r3d"
B="$DIR/Other.r3d"
"$R3D" new "$A" >/dev/null
"$R3D" new "$B" >/dev/null

# Two pieces sewn at their side, then moved point by point: 12 steps.
cat > "$DIR/draw.json" <<'JSON'
[
  {"type": "panel.create_rect", "name": "Front", "width": 300, "height": 500},
  {"type": "panel.create_rect", "name": "Back", "origin": [400, 0], "width": 300, "height": 500},
  {"type": "seam.add", "a": {"edge": "$0.created[6]"}, "b": {"edge": "$1.created[8]", "reversed": true}},
  {"type": "point.move", "point": "$0.created[1]", "to": [-20, 0]},
  {"type": "point.move", "point": "$0.created[2]", "to": [320, 0]},
  {"type": "point.move", "point": "$1.created[1]", "to": [380, 0]},
  {"type": "point.move", "point": "$1.created[2]", "to": [720, 0]},
  {"type": "panel.rename", "panel": "$0.created[0]", "name": "Front skirt"},
  {"type": "panel.rename", "panel": "$1.created[0]", "name": "Back skirt"},
  {"type": "notch.add", "edge": "$0.created[6]", "offset_mm": 120},
  {"type": "notch.add", "edge": "$1.created[8]", "offset_mm": 120},
  {"type": "panel.translate", "panel": "$1.created[0]", "delta": [50, 0]},
  {"type": "history.list"},
  {"type": "history.go_to", "at": 2},
  {"type": "project.summary"},
  {"type": "history.go_to", "at": 12},
  {"type": "history.list"}
]
JSON
echo "== 1. Twelve steps, ten back, then forward again"
"$R3D" run "$A" "$DIR/draw.json" > "$DIR/draw.out"
python3 - "$DIR/draw.out" <<'PY'
import json, sys
r = [s["result"] for s in json.load(open(sys.argv[1]))["results"]]
print("steps:", ", ".join(s["label"] for s in r[12]["steps"]), "| at", r[12]["at"])
print("back to", r[13]["at"], "->", r[14]["counts"]["panels"], "pieces, no seam yet")
print("forward to", r[15]["at"], "-> panels changed:", ", ".join(r[15]["panels"]))
PY

echo "== 2. Copy the two sewn pieces into another project"
CLIP=$("$R3D" query "$A" '{"type": "panel.copy", "panels": ["panel:2", "panel:11"]}')
printf '[{"type": "panel.paste", "clip": %s}]' "$CLIP" > "$DIR/paste.json"
"$R3D" run "$B" "$DIR/paste.json" >/dev/null
echo "pieces: $("$R3D" query "$B" '{"type": "panel.list"}' | grep -c '"id"'),"   "seams: $("$R3D" query "$B" '{"type": "seam.list"}' | grep -c '"gathered"')"

echo "== 3. The shirt drafted in every size of 36-52 (a line per size in a terminal)"
"$R3D" new "$DIR/Shirt.r3d" >/dev/null
cat > "$DIR/shirt.json" <<'JSON'
[
  {"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"},
  {"type": "template.apply", "template": "shirt"}
]
JSON
time "$R3D" run "$DIR/Shirt.r3d" "$DIR/shirt.json" >/dev/null
"$R3D" query "$DIR/Shirt.r3d" '{"type": "grading.check"}' | grep '"ok"'
