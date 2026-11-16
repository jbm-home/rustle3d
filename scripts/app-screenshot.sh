#!/usr/bin/env bash
# Capture the desktop app window (debug build) to a PNG, for visual review.
#   [R3D_SHOT_DELAY=seconds] scripts/app-screenshot.sh out.png [--dart-define=K=V ...]
# Shows the start screen, or with --dart-define=R3D_DEMO=<template> a fresh
# project from that template; captures only the Rustle3D window.
# Needs the terminal to have the macOS "Screen Recording" permission.
set -euo pipefail
out="$1"; shift
cd "$(dirname "$0")/../app"
sandbox_tmp="$HOME/Library/Containers/dev.rustle3d.rustle3d/Data/tmp"

pkill -f "Rustle3D.app/Contents/MacOS/Rustle3D" 2>/dev/null || true
rm -rf "$sandbox_tmp/rustle3d-demo" "$sandbox_tmp/rustle3d-window-id"
flutter build macos --debug "$@" >/dev/null
open build/macos/Build/Products/Debug/Rustle3D.app
for _ in $(seq 1 40); do
  [ -s "$sandbox_tmp/rustle3d-window-id" ] && break
  sleep 0.25
done
sleep "${R3D_SHOT_DELAY:-1.5}" # let the first frames render (or the cloth drape)
screencapture -x -o -l "$(cat "$sandbox_tmp/rustle3d-window-id")" "$out"
pkill -f "Rustle3D.app/Contents/MacOS/Rustle3D" || true
echo "$out"
