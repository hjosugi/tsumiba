# License

tsumiba is released under the **MIT License**.

```text
MIT License

Copyright (c) 2026 hjosugi

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Third-party software

tsumiba is built from open-source Rust crates, each under its own licence (mostly MIT and
Apache-2.0); `Cargo.lock` lists them all. The program embeds egui's default fonts (Hack,
Ubuntu Light, Noto Emoji and an emoji icon font), each under its own free licence; see the
[egui repository](https://github.com/emilk/egui) for details. Japanese text uses a font
already installed on your system; none is bundled.

## Credits

- [Unclutter](https://unclutterapp.com/) by Softwarebay: the idea of a panel at the top of
  the screen for notes and files. macOS only, closed source.
- [yeet](https://github.com/hjosugi/yeet): a drag-and-drop shelf for Wayland and Windows, from
  which this project's packaging and documentation habits come.
- [Yoink](https://eternalstorms.at/yoink/mac/), [DropPoint](https://github.com/GameGodS3/DropPoint)
  and [dragon](https://github.com/mwh/dragon): other takes on the file shelf.
