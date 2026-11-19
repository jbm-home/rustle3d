#!/usr/bin/env bash
# M15 demo: game-ready export. A script (or an agent) dresses the women's
# M body in a striped T-shirt and drapes it headless, then exports it for
# a game engine: three levels of detail (100 %, 50 %, 20 % of the
# triangles, the pieces' edges kept), the full drape's wrinkles baked
# into a normal map for each lighter level, the stripes baked into an
# atlas, and the T-shirt skinned to the avatar's 24-bone skeleton with the
# walk, turn and raise-arms motions as animations, the body included.
# Then the same for Unreal, as a 1.5 mm shell. Both files are checked by
# the Khronos glTF Validator when Node.js is there. In the app: Export ›
# Game engine (glTF)…
# Run from the repository root: ./demos/m15/demo.sh
set -euo pipefail

cargo build --quiet --release -p r3d-cli
R3D=./target/release/r3d
DIR="$(mktemp -d)"
OUT="$DIR/tee.r3d"

"$R3D" new "$OUT" >/dev/null
cat > "$DIR/setup.json" <<'JSON'
[
  {"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
   "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}},
  {"type": "template.apply", "template": "t_shirt"}
]
JSON
"$R3D" run "$OUT" "$DIR/setup.json" >/dev/null
"$R3D" texture "$OUT" stripes >/dev/null

echo "== 1. Drape the striped T-shirt (150 frames)"
"$R3D" simulate "$OUT" --frames 150 | grep -E '"max_seam_gap_mm"' | tr -d ' ,'

echo "== 2. Game-ready GLB: LODs, normal maps, skinned to the walking avatar"
"$R3D" export "$OUT" --format game --avatar \
  | tr -d '\n '
echo

echo "== 3. For Unreal, with thickness instead of double-sided faces"
"$R3D" export "$OUT" --format game --preset unreal --shell --thickness-mm 1.5 \
  | tr -d '\n '
echo

if command -v node >/dev/null; then
  echo "== 4. Khronos glTF Validator"
  tools="target/gltf-validator"
  if [ ! -d "$tools/node_modules/gltf-validator" ]; then
    mkdir -p "$tools"
    (cd "$tools" && npm install --no-save --silent gltf-validator@2.0.0-dev.3.10 >/dev/null)
  fi
  cp scripts/validate-gltf.mjs "$tools/"
  node "$tools/validate-gltf.mjs" "$OUT"/exports/*.glb
fi
ls -1 "$OUT"/exports/
