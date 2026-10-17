# tsumiba documentation site

Built with [mdBook](https://rust-lang.github.io/mdBook/) and published to
<https://hjosugi.github.io/tsumiba/>.

```text
docs-site/
├── landing/            the front page (index.html, style.css, app.js, screenshots/)
├── en/                 the English book  (book.toml, src/*.md)
├── ja/                 the Japanese book (same pages, same heading ids)
└── theme/              CSS and JS shared by both books
```

```sh
scripts/build-docs-site.sh           # -> target/docs-site/{index.html,en/,ja/}
node scripts/check-docs-links.mjs    # broken links/anchors, remote resources, en/ja parity
```

- **Edit the originals.** The build script copies `theme/` and the screenshots each book mentions
  into `en/` and `ja/` (git-ignored), because mdBook only reads inside a book's own directory.
- **Same pages, same ids.** Every page and every heading id must exist in both languages; the
  language switch carries the `#anchor` across. Use `{#id}` on Japanese headings to match English.
- **No third-party requests**: no web fonts, scripts or images from elsewhere. The checker enforces it.
- **Screenshots** in `landing/screenshots/` are of the real app (Linux/X11, a virtual display),
  1040 px wide panels at the default size. Do not replace them with mock-ups.
- The status tables (*Platform support*, the front page) state what has actually been verified.
  Keep them honest when something changes.
