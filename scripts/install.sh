#!/bin/sh
# Install tsumiba from a GitHub release (Linux and macOS).
#
#   curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version v0.1.0 --prefix /usr/local
#
# Downloads the archive for this machine, checks it against the release's
# SHA256SUMS.txt, and installs it without needing root (into ~/.local on Linux;
# into ~/Applications on macOS, with a `tsumiba` command in ~/.local/bin).
#
# TSUMIBA_BASE_URL overrides the release location (used by the test suite).
set -eu

repo="hjosugi/tsumiba"
base="${TSUMIBA_BASE_URL:-https://github.com/$repo/releases}"
version="latest"
prefix="${HOME}/.local"

while [ $# -gt 0 ]; do
  case "$1" in
    --version) version="$2"; shift 2 ;;
    --prefix) prefix="$2"; shift 2 ;;
    -h|--help)
      echo "usage: install.sh [--version vX.Y.Z] [--prefix DIR]"
      exit 0 ;;
    *) echo "install.sh: unknown option: $1" >&2; exit 2 ;;
  esac
done

say() { printf '%s\n' "$*"; }
die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

fetch() { # fetch <url> <dest>
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$1" -O "$2"
  else
    die "need curl or wget"
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d' ' -f1
  else die "need sha256sum or shasum"; fi
}

case "$(uname -s)" in
  Linux)  platform=linux ;;
  Darwin) platform=macos ;;
  *) die "unsupported system: $(uname -s). On Windows use install.ps1." ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) die "unsupported architecture: $(uname -m)" ;;
esac

if [ "$platform" = macos ]; then
  pattern="macos-universal.zip"
else
  pattern="linux-${arch}.tar.gz"
fi

if [ "$version" = latest ]; then
  release_url="$base/latest/download"
else
  release_url="$base/download/$version"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

say "Looking up the release ($version)..."
fetch "$release_url/SHA256SUMS.txt" "$work/SHA256SUMS.txt" || die "could not download SHA256SUMS.txt from $release_url"
line="$(grep -E "[ *]tsumiba-[0-9][^ ]*-${pattern}\$" "$work/SHA256SUMS.txt" | head -n 1 || true)"
[ -n "$line" ] || die "this release has no $pattern archive"
expected="$(printf '%s' "$line" | cut -d' ' -f1)"
asset="$(printf '%s' "$line" | sed -E 's/^[^ ]+ [ *]?//')"

say "Downloading $asset..."
fetch "$release_url/$asset" "$work/$asset" || die "could not download $asset"
actual="$(sha256_of "$work/$asset")"
[ "$actual" = "$expected" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"
say "Checksum OK."

if [ "$platform" = linux ]; then
  tar -xzf "$work/$asset" -C "$work"
  dir="$work/$(basename "$asset" .tar.gz)"
  [ -x "$dir/install.sh" ] || die "the archive has no install.sh"
  "$dir/install.sh" --prefix "$prefix"
  say "Run \`tsumiba\` to start it, or find it in your application menu."
else
  apps="${HOME}/Applications"
  mkdir -p "$apps" "$prefix/bin"
  rm -rf "$apps/Tsumiba.app"
  unzip -q "$work/$asset" -d "$apps"
  # The user asked for this install, so clear the download quarantine flag
  # instead of making them find the right-click > Open dance.
  xattr -dr com.apple.quarantine "$apps/Tsumiba.app" 2>/dev/null || true
  ln -sf "$apps/Tsumiba.app/Contents/MacOS/tsumiba" "$prefix/bin/tsumiba"
  say "Installed $apps/Tsumiba.app and $prefix/bin/tsumiba."
  say "Open it from Launchpad or run \`tsumiba\`."
  case ":$PATH:" in *":$prefix/bin:"*) ;; *) say "note: $prefix/bin is not on your PATH" ;; esac
fi
