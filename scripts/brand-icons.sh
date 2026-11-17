#!/usr/bin/env bash
# Renders the app icons (macOS, Windows, Linux) and the social preview from
# the SVG masters in assets/brand/. With the website's folder as argument,
# also writes the site's favicon, Apple touch icon and manifest icons.
# Needs rsvg-convert (librsvg2-bin) and ImageMagick 7 (magick).
#
#   ./scripts/brand-icons.sh [../rustle3dweb]
set -euo pipefail
cd "$(dirname "$0")/.."
B=assets/brand
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT

# A square-cornered tile, for platforms that apply their own mask (Apple
# touch icon, maskable web icon): the cloth stays inside the safe zone.
sed 's/ rx="228"//' "$B/mark.svg" >"$T/square.svg"

png() { rsvg-convert -w "$2" -h "$2" "$1" -o "$3"; }
# Small sizes use the simplified mark (no folds, three bold stitches).
pick() { if [ "$1" -le 32 ]; then echo "$2-small.svg"; else echo "$2.svg"; fi; }

# macOS: the 824 px tile on the 1024 px canvas, with its shadow.
mac=app/macos/Runner/Assets.xcassets/AppIcon.appiconset
for s in 16 32 64 128 256 512 1024; do
  png "$B/$(pick "$s" app-icon-macos)" "$s" "$mac/app_icon_$s.png"
done

# Windows: one .ico with every size Explorer and the taskbar ask for.
ico=()
for s in 16 24 32 48 64 256; do
  png "$B/$(pick "$s" mark)" "$s" "$T/ico-$s.png"
  ico+=("$T/ico-$s.png")
done
magick "${ico[@]}" app/windows/runner/resources/app_icon.ico

# Linux: the window icon, installed next to the bundle's data.
mkdir -p app/linux/runner/resources
png "$B/mark.svg" 256 app/linux/runner/resources/app_icon.png

# GitHub social preview (Settings > Social preview).
rsvg-convert -w 1280 -h 640 "$B/social-preview.svg" -o "$B/social-preview.png"

if [ $# -ge 1 ]; then
  web="$1"
  cp "$B/mark-small.svg" "$web/src/app/icon.svg"
  for s in 16 32 48; do png "$B/$(pick "$s" mark)" "$s" "$T/fav-$s.png"; done
  magick "$T/fav-16.png" "$T/fav-32.png" "$T/fav-48.png" "$web/src/app/favicon.ico"
  png "$T/square.svg" 180 "$web/src/app/apple-icon.png"
  png "$B/mark.svg" 192 "$web/public/icon-192.png"
  png "$B/mark.svg" 512 "$web/public/icon-512.png"
  png "$T/square.svg" 512 "$web/public/icon-maskable-512.png"
fi
echo "Icons written."
