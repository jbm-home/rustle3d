#!/usr/bin/env bash
# Build the desktop app and the CLI in release mode for this machine's OS
# and package them in dist/ (unsigned):
#   macOS   dist/Rustle3D-<version>-macos.zip        (Rustle3D.app + r3d)
#   Linux   dist/Rustle3D-<version>-linux-x64.tar.gz (app bundle + r3d)
#   Windows dist/Rustle3D-<version>-windows-x64.zip  (app folder + r3d.exe; Git Bash)
# Needs Rust, Flutter and the platform toolchain (see README "Building").
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
mkdir -p dist
cargo build --release -p r3d-cli

case "$(uname -s)" in
  Darwin)
    (cd app && flutter build macos --release)
    stage="$(mktemp -d)/Rustle3D-$version"
    mkdir -p "$stage"
    cp -R app/build/macos/Build/Products/Release/Rustle3D.app "$stage/"
    cp target/release/r3d "$stage/"
    out="dist/Rustle3D-$version-macos.zip"
    rm -f "$out"
    ditto -c -k --keepParent "$stage" "$out"
    ;;
  Linux)
    (cd app && flutter build linux --release)
    stage="$(mktemp -d)/Rustle3D-$version"
    mkdir -p "$stage"
    cp -R app/build/linux/x64/release/bundle/. "$stage/"
    cp target/release/r3d "$stage/"
    out="dist/Rustle3D-$version-linux-x64.tar.gz"
    tar -C "$(dirname "$stage")" -czf "$out" "$(basename "$stage")"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    (cd app && flutter build windows --release)
    stage="$(mktemp -d)/Rustle3D-$version"
    mkdir -p "$stage"
    cp -R app/build/windows/x64/runner/Release/. "$stage/"
    cp target/release/r3d.exe "$stage/"
    out="dist/Rustle3D-$version-windows-x64.zip"
    rm -f "$out"
    (cd "$(dirname "$stage")" && powershell -NoProfile -Command \
      "Compress-Archive -Path '$(basename "$stage")' -DestinationPath '$(cygpath -w "$OLDPWD/$out")'")
    ;;
  *)
    echo "unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac
echo "$out"
