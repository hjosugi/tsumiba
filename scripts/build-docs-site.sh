#!/usr/bin/env bash
# Build the documentation site into target/docs-site:
#
#   target/docs-site/index.html      the front page (docs-site/landing)
#   target/docs-site/en/             the English book (docs-site/en)
#   target/docs-site/ja/             the Japanese book (docs-site/ja)
#
# Needs mdbook (`nix develop` has it). Works from any directory.
#
# The books share one theme and the screenshots, which live in one place each;
# this script copies them into every book before building so that mdBook, which
# only reads inside a book's own directory, can find them. The copies are
# git-ignored.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
site="$root/docs-site"
out="$root/target/docs-site"

command -v mdbook >/dev/null 2>&1 || {
  echo "build-docs-site: mdbook not found. Run this inside 'nix develop', or install mdbook." >&2
  exit 1
}

rm -rf "$out"
mkdir -p "$out"

# One source for the icon: the app's own artwork.
cp "$root/assets/tsumiba.svg" "$site/landing/icon.svg"

for lang in en ja; do
  book="$site/$lang"

  # Shared theme, and the app icon as the favicon (mdBook picks up theme/favicon.*).
  rm -rf "$book/theme"
  mkdir -p "$book/theme"
  cp "$site"/theme/* "$book/theme/"
  cp "$root/assets/tsumiba.svg" "$book/theme/favicon.svg"
  cp "$root/assets/icons/tsumiba-32.png" "$book/theme/favicon.png"

  # Only the screenshots this book's pages actually mention.
  rm -rf "$book/src/images"
  mkdir -p "$book/src/images"
  { grep -rhoE 'images/[A-Za-z0-9._-]+\.png' "$book/src" --include='*.md' || true; } | sort -u |
    while read -r ref; do
      cp "$site/landing/screenshots/$(basename "$ref")" "$book/src/images/" || {
        echo "build-docs-site: $lang book mentions $ref, which is not in docs-site/landing/screenshots" >&2
        exit 1
      }
    done

  log="$(mktemp)"
  mdbook build "$book" 2>&1 | tee "$log"
  # mdBook reports broken links and missing files as warnings; treat them as errors.
  if grep -Eq '\b(WARN|ERROR)\b' "$log"; then
    echo "build-docs-site: mdbook reported warnings or errors for the $lang book (see above)" >&2
    rm -f "$log"
    exit 1
  fi
  rm -f "$log"
done

cp -R "$site/landing/." "$out/"
# GitHub Pages: serve the files as they are.
touch "$out/.nojekyll"

echo
echo "Docs site is ready at: $out"
(cd "$out" && find . -maxdepth 2 \( -name '*.html' -o -name '*.css' -o -name '*.png' -o -name '*.svg' \) -not -path './*/*/*' |
  sort | head -80)
