#!/usr/bin/env bash
# M9 demo: the draped garment leaves Rustle3D for a game engine or Blender,
# and an agent works on the project open in the app.
# Run from the repository root: ./demos/m9/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
OUT="$(mktemp -d)/Skirt.r3d"

"$R3D" new "$OUT" >/dev/null
"$R3D" run "$OUT" demos/m10/body.json >/dev/null
"$R3D" run "$OUT" /dev/stdin >/dev/null <<'JSON'
[{"type": "template.apply", "template": "a_line_skirt"},
 {"type": "garment.set_fabric", "fabric": "denim"}]
JSON

echo "== 1. Drape it headless (3 s)"
"$R3D" simulate "$OUT" --frames 180 | grep -E '"(top_m|bottom_m|max_seam_gap_mm)"'

echo "== 2. Export the draped skirt: glTF with the body, and OBJ"
"$R3D" export "$OUT" --format glb --avatar | grep -E '"(path|drape|pieces)"'
"$R3D" export "$OUT" --format obj | grep -E '"path"'
ls -lh "$OUT"/exports/

if command -v node >/dev/null; then
  echo "== 3. Check the glTF with the Khronos validator"
  ./scripts/validate-exports.sh
fi

cat <<'TEXT'
== 4. Live, with the app
In the app, open a project and choose Agents > Allow AI agents. Then give
an MCP client this server command:

    r3d mcp --attach

Ask the agent to make the skirt 5 cm shorter (edge_lengthen on the hem):
the pattern and the drape change in the app as it works, and Cmd/Ctrl+Z
undoes it. Agents > Disconnect the agent ends the session.
TEXT
