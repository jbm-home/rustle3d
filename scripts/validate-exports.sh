#!/usr/bin/env bash
# Export each template draped (briefly) with the avatar as GLB and check
# the files with the Khronos glTF Validator; the T-shirt and the dress in
# a print (built-in, JPEG image), so embedded textures are checked too.
# Needs Node.js; the validator is installed once into target/gltf-validator.
set -euo pipefail
cd "$(dirname "$0")/.."
tools="target/gltf-validator"
if [ ! -d "$tools/node_modules/gltf-validator" ]; then
  mkdir -p "$tools"
  (cd "$tools" && npm install --no-save --silent gltf-validator@2.0.0-dev.3.10 >/dev/null)
fi
cp scripts/validate-gltf.mjs "$tools/"
cargo build -q -p r3d-cli
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
files=()
for template in t_shirt dress a_line_skirt tote_bag; do
  project="$work/$template.r3d"
  target/debug/r3d new "$project" >/dev/null
  cat > "$work/$template.json" <<JSON
[{"type":"avatar.set","preset":"Women M","body":{"masculinity":0,"height_cm":168,"bust_cm":90,"waist_cm":73,"hips_cm":98}},
 {"type":"template.apply","template":"$template"}]
JSON
  target/debug/r3d run "$project" "$work/$template.json" >/dev/null
  case "$template" in
    t_shirt) target/debug/r3d texture "$project" stripes >/dev/null ;;
    dress) target/debug/r3d texture "$project" crates/r3d-io/tests/data/two.jpg --width-mm 40 >/dev/null ;;
  esac
  cat > "$work/$template-export.json" <<JSON
[{"type":"simulate","frames":5},
 {"type":"export.gltf","path":"out.glb","avatar":true}]
JSON
  target/debug/r3d run "$project" "$work/$template-export.json" >/dev/null
  files+=("$project/out.glb")
done
node "$tools/validate-gltf.mjs" "${files[@]}"
