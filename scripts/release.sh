#!/usr/bin/env bash
# Build the desktop app and the CLI in release mode for this machine's OS
# and package them in dist/ (unsigned):
#   macOS   dist/Rustle3D-<version>-macos.zip        (Rustle3D.app + r3d)
#   Linux   dist/Rustle3D-<version>-linux-x64.tar.gz (app bundle + r3d)
#   Windows dist/Rustle3D-<version>-windows-x64.zip  (app folder + r3d.exe; Git Bash)
# Each package also holds what the GPL and the dependencies' licences ask
# for: LICENSE, THIRD_PARTY_NOTICES.md, the licences of the Rust crates
# (LICENSES-rust.html, by cargo-about) and of the Flutter packages
# (LICENSES-flutter.txt), and README.txt with the source code's address.
# Needs Rust, Flutter, cargo-about (cargo install cargo-about --locked
# --features cli), Python 3 and the platform toolchain (see README "Building").
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
command -v cargo-about >/dev/null || { echo "cargo-about is missing (cargo install cargo-about --locked --features cli)" >&2; exit 1; }
python="$(command -v python3 || command -v python || true)"
[ -n "$python" ] || { echo "Python 3 is missing" >&2; exit 1; }
mkdir -p dist
cargo build --release -p r3d-cli
stage="$(mktemp -d)/Rustle3D-$version"
mkdir -p "$stage"

case "$(uname -s)" in
  Darwin)
    (cd app && flutter build macos --release)
    cp -R app/build/macos/Build/Products/Release/Rustle3D.app "$stage/"
    cp target/release/r3d "$stage/"
    ;;
  Linux)
    (cd app && flutter build linux --release)
    cp -R app/build/linux/x64/release/bundle/. "$stage/"
    cp target/release/r3d "$stage/"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    (cd app && flutter build windows --release)
    cp -R app/build/windows/x64/runner/Release/. "$stage/"
    cp target/release/r3d.exe "$stage/"
    ;;
  *)
    echo "unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

# Licences and source code.
cp LICENSE THIRD_PARTY_NOTICES.md "$stage/"
cargo about generate about.hbs -o "$stage/LICENSES-rust.html"
notices="$(find "$stage" -name NOTICES.Z | head -n 1)"
[ -n "$notices" ] || { echo "Flutter's NOTICES.Z not found in the build" >&2; exit 1; }
"$python" -c 'import sys, zlib; sys.stdout.buffer.write(zlib.decompress(open(sys.argv[1], "rb").read(), 47))' \
  "$notices" > "$stage/LICENSES-flutter.txt"
cat > "$stage/README.txt" <<EOF
Rustle3D $version - 3D garment design: draft patterns, sew, drape.
https://rustle3d.com

Rustle3D is free software under the GNU General Public License, version 3
or later (LICENSE). Its source code, for this exact version:
https://github.com/jbm-home/rustle3d/tree/v$version

Code and assets from other projects: THIRD_PARTY_NOTICES.md.
Licences of the libraries it is built with: LICENSES-rust.html (Rust
crates) and LICENSES-flutter.txt (Flutter and its packages).

r3d is the command-line tool (r3d --help). The packages are not signed
yet: the first launch needs one extra step, explained in the user guide,
https://rustle3d.com/en/guide/ (Installing).
EOF

case "$(uname -s)" in
  Darwin)
    out="dist/Rustle3D-$version-macos.zip"
    rm -f "$out"
    ditto -c -k --keepParent "$stage" "$out"
    ;;
  Linux)
    out="dist/Rustle3D-$version-linux-x64.tar.gz"
    tar -C "$(dirname "$stage")" -czf "$out" "$(basename "$stage")"
    ;;
  *)
    out="dist/Rustle3D-$version-windows-x64.zip"
    rm -f "$out"
    (cd "$(dirname "$stage")" && powershell -NoProfile -Command \
      "Compress-Archive -Path '$(basename "$stage")' -DestinationPath '$(cygpath -w "$OLDPWD/$out")'")
    ;;
esac
echo "$out"
