#!/usr/bin/env bash
# Assemble the Linux release archive.
#
#   scripts/package-linux.sh <version> <arch> <path/to/tsumiba>
#
# Produces dist/tsumiba-<version>-linux-<arch>.tar.gz containing
#   bin/tsumiba, share/applications/tsumiba.desktop, share/icons/hicolor/...,
#   LICENSE, README.md and an install.sh that copies it all under a prefix.
set -euo pipefail

version="${1:?version}"; arch="${2:?arch, e.g. x86_64}"; binary="${3:?path to the tsumiba binary}"
root="$(cd "$(dirname "$0")/.." && pwd)"
name="tsumiba-${version}-linux-${arch}"
stage="$root/dist/$name"

rm -rf "$stage"
install -Dm755 "$binary" "$stage/bin/tsumiba"
install -Dm644 "$root/packaging/linux/tsumiba.desktop" "$stage/share/applications/tsumiba.desktop"
for size in 16 32 48 64 128 256 512; do
  install -Dm644 "$root/assets/icons/tsumiba-$size.png" "$stage/share/icons/hicolor/${size}x${size}/apps/tsumiba.png"
done
install -Dm644 "$root/assets/tsumiba.svg" "$stage/share/icons/hicolor/scalable/apps/tsumiba.svg"
install -Dm644 "$root/LICENSE" "$stage/LICENSE"
install -Dm644 "$root/README.md" "$stage/README.md"

cat > "$stage/install.sh" <<'INSTALL'
#!/bin/sh
# Install tsumiba from this archive. No root needed by default.
#   ./install.sh                 -> ~/.local
#   ./install.sh --prefix /usr/local
set -eu
prefix="${HOME}/.local"
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) prefix="$2"; shift 2 ;;
    -h|--help) echo "usage: $0 [--prefix DIR]"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
here="$(cd "$(dirname "$0")" && pwd)"
install -Dm755 "$here/bin/tsumiba" "$prefix/bin/tsumiba"
( cd "$here/share" && find . -type f | while read -r f; do
    install -Dm644 "$f" "$prefix/share/$f"
  done )
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" 2>/dev/null || true
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$prefix/share/applications" 2>/dev/null || true
echo "installed tsumiba to $prefix"
case ":$PATH:" in *":$prefix/bin:"*) ;; *) echo "note: $prefix/bin is not on your PATH" ;; esac
INSTALL
chmod +x "$stage/install.sh"

mkdir -p "$root/dist"
tar -C "$root/dist" -czf "$root/dist/$name.tar.gz" "$name"
rm -rf "$stage"
echo "$root/dist/$name.tar.gz"
