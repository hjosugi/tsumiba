# ライセンス {#license}

tsumiba は **MIT ライセンス**で公開されています。以下は英語の原文です(法的な効力を持つのは原文です)。

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

## サードパーティのソフトウェア {#third-party-software}

tsumiba は、オープンソースの Rust クレートから作られており、それぞれが固有のライセンス(主に MIT と
Apache-2.0)を持っています。すべては `Cargo.lock` に一覧があります。プログラムには egui の既定のフォント
(Hack、Ubuntu Light、Noto Emoji、絵文字アイコンのフォント)が埋め込まれており、それぞれ固有の自由な
ライセンスです。詳細は [egui のリポジトリ](https://github.com/emilk/egui)を参照してください。日本語の表示には
システムにすでに入っているフォントを使い、同梱しているものはありません。

## クレジット {#credits}

- [Unclutter](https://unclutterapp.com/)(Softwarebay):画面の上端のパネルにメモとファイルを置く、という
  発想の元。macOS 専用、クローズドソース。
- [yeet](https://github.com/hjosugi/yeet):Wayland と Windows 向けのドラッグ&ドロップの置き場。
  このプロジェクトのパッケージングとドキュメントの流儀の元になっています。
- [Yoink](https://eternalstorms.at/yoink/mac/)、[DropPoint](https://github.com/GameGodS3/DropPoint)、
  [dragon](https://github.com/mwh/dragon):ファイルの置き場の、ほかのアプローチ。
