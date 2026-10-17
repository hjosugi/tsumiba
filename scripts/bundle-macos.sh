#!/usr/bin/env bash
# Build Tsumiba.app (and a .zip and .dmg of it) from a macOS binary.
#
#   scripts/bundle-macos.sh <version> <path/to/tsumiba>
#
# Runs on macOS only (iconutil, codesign, hdiutil, ditto). The binary may be
# universal (lipo) or single-architecture.
set -euo pipefail

version="${1:?version}"; binary="${2:?path to the tsumiba binary}"
root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/dist"
app="$out/Tsumiba.app"

# iconutil insists that the directory be named *.iconset.
iconset="$out/tsumiba.iconset"
rm -rf "$app" "$iconset" "$out/dmg"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$iconset"
install -m755 "$binary" "$app/Contents/MacOS/tsumiba"

# Icon: an .iconset needs each size at 1x and 2x.
icons="$root/assets/icons"
cp "$icons/tsumiba-16.png"   "$iconset/icon_16x16.png"
cp "$icons/tsumiba-32.png"   "$iconset/icon_16x16@2x.png"
cp "$icons/tsumiba-32.png"   "$iconset/icon_32x32.png"
cp "$icons/tsumiba-64.png"   "$iconset/icon_32x32@2x.png"
cp "$icons/tsumiba-128.png"  "$iconset/icon_128x128.png"
cp "$icons/tsumiba-256.png"  "$iconset/icon_128x128@2x.png"
cp "$icons/tsumiba-256.png"  "$iconset/icon_256x256.png"
cp "$icons/tsumiba-512.png"  "$iconset/icon_256x256@2x.png"
cp "$icons/tsumiba-512.png"  "$iconset/icon_512x512.png"
cp "$icons/tsumiba-1024.png" "$iconset/icon_512x512@2x.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/tsumiba.icns"
rm -rf "$iconset"

# LSUIElement: an agent app, so no Dock icon and no menu bar of its own; it
# lives in the menu bar's status area and slides in from the top edge.
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>tsumiba</string>
  <key>CFBundleDisplayName</key><string>tsumiba</string>
  <key>CFBundleIdentifier</key><string>io.github.hjosugi.tsumiba</string>
  <key>CFBundleExecutable</key><string>tsumiba</string>
  <key>CFBundleIconFile</key><string>tsumiba</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT License. Copyright (c) 2026 hjosugi</string>
</dict>
</plist>
PLIST

# Ad-hoc signature: not a Developer ID, but it keeps Apple Silicon from refusing
# to run an unsigned bundle. Gatekeeper will still ask the user to confirm once.
codesign --force --deep --sign - "$app"

base="tsumiba-${version}-macos-universal"
ditto -c -k --keepParent "$app" "$out/$base.zip"

mkdir -p "$out/dmg"
cp -R "$app" "$out/dmg/"
ln -s /Applications "$out/dmg/Applications"
# hdiutil occasionally fails with "Resource busy" on CI runners; try again.
for attempt in 1 2 3 4; do
  if hdiutil create -volname "tsumiba ${version}" -srcfolder "$out/dmg" -ov -format UDZO "$out/$base.dmg" >/dev/null; then
    break
  fi
  [ "$attempt" = 4 ] && { echo "hdiutil failed four times" >&2; exit 1; }
  sleep 5
done
rm -rf "$out/dmg"
echo "$out/$base.zip"
echo "$out/$base.dmg"
