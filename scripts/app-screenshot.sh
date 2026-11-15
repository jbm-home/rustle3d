#!/usr/bin/env bash
# Capture the desktop app window (debug build) to a PNG, for visual review.
#   scripts/app-screenshot.sh out.png [--dart-define=K=V ...]
# Starts from a fresh demo project and captures only the Rustle3D window.
# Needs the terminal to have the macOS "Screen Recording" permission.
set -euo pipefail
out="$1"; shift
cd "$(dirname "$0")/../app"
sandbox_tmp="$HOME/Library/Containers/dev.rustle3d.rustle3d/Data/tmp"

pkill -f "rustle3d.app/Contents/MacOS/rustle3d" 2>/dev/null || true
rm -rf "$sandbox_tmp/rustle3d-spike.r3d" "$sandbox_tmp/rustle3d-window-id"
flutter build macos --debug "$@" >/dev/null
open build/macos/Build/Products/Debug/rustle3d.app
for _ in $(seq 1 40); do
  [ -s "$sandbox_tmp/rustle3d-window-id" ] && break
  sleep 0.25
done
sleep 1.5 # let the first frames render
screencapture -x -o -l "$(cat "$sandbox_tmp/rustle3d-window-id")" "$out"
pkill -f "rustle3d.app/Contents/MacOS/rustle3d" || true
echo "$out"
