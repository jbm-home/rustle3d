#!/usr/bin/env bash
# Build the desktop app and the CLI in release mode for this machine's OS
# and package them in dist/ (unsigned):
#   macOS   dist/Rustle3D-<version>-macos-universal.dmg   (Intel and Apple
#           silicon; Rustle3D.app with r3d in Contents/MacOS, to drag to
#           Applications)
#           dist/Rustle3D-<version>-macos-universal.zip   (portable: the same
#           app, to unpack and run where you like)
#   Windows dist/Rustle3D-<version>-windows-x64-setup.exe (installer, Inno
#           Setup; app folder + r3d.exe; Git Bash)
#           dist/Rustle3D-<version>-windows-x64.zip       (portable: the same
#           folder, to unpack and run without installing)
#   Linux   dist/Rustle3D-<version>-linux-x64.tar.gz      (app bundle + r3d)
# Each package also holds what the GPL and the dependencies' licences ask
# for: LICENSE, THIRD_PARTY_NOTICES.md, the licences of the Rust crates
# (LICENSES-rust.html, by cargo-about) and of the Flutter packages
# (LICENSES-flutter.txt), and README.txt with the source code's address
# (inside Rustle3D.app/Contents/Resources on macOS).
# Needs Rust, Flutter, cargo-about (cargo install cargo-about --locked
# --features cli), Python 3 and the platform toolchain (see
# docs/building.md); on macOS the x86_64 and aarch64 Rust targets (added with
# rustup when missing); on Windows Inno Setup 6 (ISCC on PATH or in its
# default folder).
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
command -v cargo-about >/dev/null || { echo "cargo-about is missing (cargo install cargo-about --locked --features cli)" >&2; exit 1; }
python="$(command -v python3 || command -v python || true)"
[ -n "$python" ] || { echo "Python 3 is missing" >&2; exit 1; }
os="$(uname -s)"
case "$os" in
  MINGW*|MSYS*|CYGWIN*)
    iscc="$(command -v iscc || true)"
    for candidate in "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" "/c/Program Files/Inno Setup 6/ISCC.exe" \
      "$(cygpath -u "${LOCALAPPDATA:-C:/}")/Programs/Inno Setup 6/ISCC.exe"; do
      if [ -z "$iscc" ] && [ -f "$candidate" ]; then iscc="$candidate"; fi
    done
    [ -n "$iscc" ] || { echo "Inno Setup 6 is missing (winget install JRSoftware.InnoSetup)" >&2; exit 1; }
    ;;
esac
mkdir -p dist
work="$(mktemp -d)"
stage="$work/Rustle3D-$version"
legal="$work/legal"
mkdir -p "$stage" "$legal"

# The command-line tool; universal (x86_64 + arm64) on macOS.
case "$os" in
  Darwin)
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    cargo build --release -p r3d-cli --target aarch64-apple-darwin
    cargo build --release -p r3d-cli --target x86_64-apple-darwin
    r3d="$work/r3d"
    lipo -create -output "$r3d" target/aarch64-apple-darwin/release/r3d target/x86_64-apple-darwin/release/r3d
    ;;
  MINGW*|MSYS*|CYGWIN*)
    cargo build --release -p r3d-cli
    r3d=target/release/r3d.exe
    ;;
  *)
    cargo build --release -p r3d-cli
    r3d=target/release/r3d
    ;;
esac

# The app.
case "$os" in
  Darwin)
    # A release build has both architectures (Xcode's ARCHS; cargokit builds
    # the Rust core for each): check every binary has them.
    (cd app && flutter build macos --release)
    built=app/build/macos/Build/Products/Release
    while IFS= read -r -d '' f; do
      if file -b "$f" | grep -q Mach-O; then
        archs=" $(lipo -archs "$f") "
        case "$archs" in *" x86_64 "*) ;; *) echo "$f has no x86_64 ($archs)" >&2; exit 1 ;; esac
        case "$archs" in *" arm64 "*) ;; *) echo "$f has no arm64 ($archs)" >&2; exit 1 ;; esac
      fi
    done < <(find "$built/Rustle3D.app" -type f -print0)
    ;;
  Linux)
    (cd app && flutter build linux --release)
    built=app/build/linux/x64/release/bundle
    ;;
  *)
    (cd app && flutter build windows --release)
    built=app/build/windows/x64/runner/Release
    ;;
esac

# Licences and source code.
cp LICENSE THIRD_PARTY_NOTICES.md "$legal/"
cargo about generate about.hbs -o "$legal/LICENSES-rust.html"
notices="$(find "$built" -name NOTICES.Z | head -n 1)"
[ -n "$notices" ] || { echo "Flutter's NOTICES.Z not found in the build" >&2; exit 1; }
"$python" -c 'import sys, zlib; sys.stdout.buffer.write(zlib.decompress(open(sys.argv[1], "rb").read(), 47))' \
  "$notices" > "$legal/LICENSES-flutter.txt"
case "$os" in
  Darwin) cli_where="It is inside the app: Rustle3D.app/Contents/MacOS/r3d" ;;
  Linux) cli_where="It is in this folder" ;;
  *) cli_where="It is in the folder Rustle3D is installed in (or unpacked in)" ;;
esac
cat > "$legal/README.txt" <<EOF
Rustle3D $version - 3D garment design: draft patterns, sew, drape.
https://rustle3d.com

Rustle3D is free software under the GNU General Public License, version 3
or later (LICENSE). Its source code, for this exact version:
https://github.com/jbm-home/rustle3d/tree/v$version

Code and assets from other projects: THIRD_PARTY_NOTICES.md.
Licences of the libraries it is built with: LICENSES-rust.html (Rust
crates) and LICENSES-flutter.txt (Flutter and its packages).

r3d is the command-line tool (r3d --help). $cli_where.

The packages are not signed yet: the first launch needs one extra step,
explained in the user guide, https://rustle3d.com/en/guide/ (Installing).
EOF

case "$os" in
  Darwin)
    # Rustle3D.app with r3d and the licences inside, signed again ad hoc
    # (adding files breaks the seal Xcode made; Apple silicon refuses to run
    # unsigned code), and a link to Applications to drag it onto.
    app="$stage/Rustle3D.app"
    cp -R "$built/Rustle3D.app" "$app"
    cp "$r3d" "$app/Contents/MacOS/r3d"
    cp "$legal"/* "$app/Contents/Resources/"
    codesign --force --sign - "$app/Contents/MacOS/r3d"
    codesign --force --sign - --preserve-metadata=entitlements,requirements,flags,runtime "$app"
    codesign --verify --deep --strict "$app"
    cp "$legal/README.txt" "$stage/"
    # Portable: the folder as it is now; ditto keeps the app's symbolic
    # links and signature.
    portable="dist/Rustle3D-$version-macos-universal.zip"
    rm -f "$portable"
    ditto -c -k --keepParent "$stage" "$portable"
    echo "$portable"
    ln -s /Applications "$stage/Applications"
    out="dist/Rustle3D-$version-macos-universal.dmg"
    rm -f "$out"
    hdiutil create -volname "Rustle3D $version" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$out"
    ;;
  Linux)
    cp -R "$built/." "$stage/"
    cp "$r3d" "$legal"/* "$stage/"
    out="dist/Rustle3D-$version-linux-x64.tar.gz"
    tar -C "$work" -czf "$out" "$(basename "$stage")"
    ;;
  *)
    cp -R "$built/." "$stage/"
    cp "$r3d" "$legal"/* "$stage/"
    # Portable: the installer's folder, zipped and compressed with Python
    # (Git Bash has no zip; python -m zipfile does not compress).
    portable="dist/Rustle3D-$version-windows-x64.zip"
    rm -f "$portable"
    "$python" -c '
import os, sys, zipfile
root, out = sys.argv[1], sys.argv[2]
base = os.path.dirname(root)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for d, _, files in os.walk(root):
        for f in files:
            p = os.path.join(d, f)
            z.write(p, os.path.relpath(p, base))
' "$(cygpath -w "$stage")" "$(cygpath -w "$PWD/$portable")"
    echo "$portable"
    name="Rustle3D-$version-windows-x64-setup"
    out="dist/$name.exe"
    rm -f "$out"
    # No MSYS path conversion: it would take ISCC's /Q and /D options for paths.
    MSYS2_ARG_CONV_EXCL='*' "$iscc" /Q "/DAppVersion=$version" "/DSourceDir=$(cygpath -w "$stage")" \
      "/DOutputDir=$(cygpath -w "$PWD/dist")" "/DOutputName=$name" "$(cygpath -w packaging/windows/rustle3d.iss)"
    ;;
esac
echo "$out"
