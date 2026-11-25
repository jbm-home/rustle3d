#!/usr/bin/env bash
# Open game exports (spec 15) in Godot 4 and check them as a game sees
# them (scripts/godot-check.gd): a T-shirt skinned to the built-in body
# with the avatar, its motions and a captured one; the same T-shirt as an
# Unreal shell without skin; a dress on an imported body (a scan without
# a skeleton), skinned to the skeleton placed on it.
# Godot is $GODOT, or `godot` on the PATH; otherwise Godot 4 (MIT) is
# downloaded once into target/godot (Linux x86_64 and Windows).
set -euo pipefail
cd "$(dirname "$0")/.."
version="4.4.1-stable"
godot="${GODOT:-$(command -v godot || command -v godot4 || true)}"
if [ -z "$godot" ]; then
  tools="target/godot"
  case "$(uname -s)" in
    Linux) file="Godot_v${version}_linux.x86_64" ;;
    MINGW*|MSYS*|CYGWIN*) file="Godot_v${version}_win64_console.exe"; zip="Godot_v${version}_win64.exe.zip" ;;
    *) echo "Set GODOT to a Godot 4 binary"; exit 1 ;;
  esac
  zip="${zip:-$file.zip}"
  if [ ! -x "$tools/$file" ]; then
    mkdir -p "$tools"
    curl -sSfL -o "$tools/$zip" \
      "https://github.com/godotengine/godot/releases/download/$version/$zip"
    (cd "$tools" && unzip -oq "$zip" && rm "$zip")
    chmod +x "$tools/$file"
  fi
  godot="$tools/$file"
fi
cargo build -q -p r3d-cli
r3d=target/debug/r3d
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
body='{"type":"avatar.set","preset":"Women M","body":{"masculinity":0,"height_cm":168,"bust_cm":90,"waist_cm":73,"hips_cm":98}}'
game="$work/game"
mkdir -p "$game"
touch "$game/project.godot"
cp scripts/godot-check.gd "$game/"

# Export with `r3d export --format game ARGS`, keeping its result for the
# check (`avatar` added: the body is one of the meshes).
export_game() {
  local project="$1" name="$2" avatar="$3"
  shift 3
  "$r3d" export "$project" --format game --out "$name" "$@" > "$work/$name.out"
  cp "$project/$name" "$game/"
  sed "s/^{/{\"avatar\": $avatar,/" "$work/$name.out" > "$game/$name.json"
}

tee="$work/tee.r3d"
"$r3d" new "$tee" >/dev/null
echo "[$body, {\"type\":\"template.apply\",\"template\":\"t_shirt\"}, {\"type\":\"simulate\",\"frames\":5}]" \
  > "$work/tee.json"
"$r3d" run "$tee" "$work/tee.json" >/dev/null
mkdir -p "$tee/motions"
cp crates/r3d-commands/tests/data/walk.bvh "$tee/motions/stroll.bvh"
export_game "$tee" tee.glb true --preset godot --avatar --normal-map 256
export_game "$tee" tee-shell.glb false --preset unreal --shell --no-skin --normal-map 256

# A scan: the built-in body as an OBJ, without a skeleton.
bare="$work/bare.r3d"
"$r3d" new "$bare" >/dev/null
echo "[$body]" > "$work/bare.json"
"$r3d" run "$bare" "$work/bare.json" >/dev/null
"$r3d" export "$bare" --format obj --avatar --out scan.obj >/dev/null
scan="$work/scan.r3d"
"$r3d" new "$scan" >/dev/null
"$r3d" avatar import "$scan" "$bare/scan.obj" >/dev/null
echo '[{"type":"template.apply","template":"dress"}, {"type":"simulate","frames":5}]' > "$work/scan.json"
"$r3d" run "$scan" "$work/scan.json" >/dev/null
export_game "$scan" scan.glb true --preset godot --avatar --normal-map 256

# Import (errors fail the check), then open each file.
log="$work/import.log"
"$godot" --headless --path "$game" --import >"$log" 2>&1 || { cat "$log"; exit 1; }
if grep -E "^(ERROR|SCRIPT ERROR)" "$log"; then
  echo "Godot reported errors while importing"
  exit 1
fi
"$godot" --headless --path "$game" --script res://godot-check.gd -- tee.glb tee-shell.glb scan.glb
