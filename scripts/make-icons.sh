#!/usr/bin/env bash
# Render the application icons from assets/tsumiba.svg.
# Needs ImageMagick built with librsvg. The outputs are committed, so this only
# has to be run when the artwork changes.
set -euo pipefail
cd "$(dirname "$0")/.."

out=assets/icons
mkdir -p "$out" assets/windows

for size in 16 32 48 64 128 256 512; do
  magick -background none -density 384 assets/tsumiba.svg -resize "${size}x${size}" "PNG32:$out/tsumiba-$size.png"
done

# Windows icon: the sizes Explorer asks for.
magick "$out/tsumiba-16.png" "$out/tsumiba-32.png" "$out/tsumiba-48.png" \
  "$out/tsumiba-64.png" "$out/tsumiba-128.png" "$out/tsumiba-256.png" assets/windows/tsumiba.ico

# macOS menu bar glyph (template image, 22pt at 2x).
magick -background none -density 1536 assets/tray-glyph.svg -resize 44x44 "PNG32:$out/tray-template.png"
echo "wrote $(ls "$out" | wc -l) icons"
