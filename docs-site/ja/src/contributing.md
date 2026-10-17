# 開発に参加する {#contributing}

tsumiba は意図的に小さく作られています。**メモとファイル、それだけ。**その方針を守り、軽さを保つ変更が、
一番受け入れやすいものです。リポジトリの
[CONTRIBUTING.md](https://github.com/hjosugi/tsumiba/blob/main/CONTRIBUTING.md) が正式なガイドで、
このページは短縮版と、ヘッドレスでの検証の仕組みです。

## セットアップ {#set-up}

Nix があれば、`nix develop` で Rust のツールチェーン、ライブラリ、`mdbook`、Node、ヘッドレス GUI 検証用の
ツール(仮想ディスプレイ、入力注入、スクリーンショット)が揃います。Nix が無い場合は、Rust 1.92 以降と、
Linux では `pkg-config` と `libxcb`・`libxkbcommon`・`libx11`・`wayland` の開発用ファイルが必要です。

## プルリクエストの前に {#before-a-pull-request}

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

テストが対象とするもの:

| 場所 | 内容 |
| --- | --- |
| `crates/tsumiba-core` | ストア(取り込み、修復、ゴミ箱、上限)、設定、ID、ファイル名、トークン。ディスプレイは不要 |
| `crates/tsumiba/src` | 上端の状態機械、モニターの幾何、ショートカットの解析、スライドのタイミング、整形、翻訳、色のコントラスト |
| `crates/tsumiba/tests/api.rs` | 実際のソケット上の実際のサーバーに、手書きの HTTP を送る:認証、Host と Origin の検査、CORS、アップロードとその上限、悪意あるファイル名 |
| `crates/tsumiba/tests/cli.rs` | 実際のプログラムをプロセスとして、使い捨てのホームディレクトリで動かす。起動中の `serve` に対するものも含む |
| `extension/test` | ブラウザ拡張:ユニットテスト、実際の `tsumiba serve` に対するクライアントとバックグラウンドスクリプト、そしてヘッドレス Chromium に読み込んだビルド済みの拡張。`cd extension && npm test` で実行(tsumiba のデバッグビルドが必要。`TSUMIBA_BIN` でも指定可。ブラウザのテストには `PATH` 上の `chromium`、または `CHROMIUM_BIN` が必要) |

## 画面なしでウィンドウをテストする {#testing-the-window-without-a-screen}

Linux では GUI をヘッドレスで操作できます。[対応プラットフォーム](platforms.md)の「確認済み」は、
こうして得られたものです。

```sh
cargo build --release
scripts/smoke-gui-linux.sh target/release/tsumiba    # CI でも実行される
```

実際のウィンドウを仮想 X サーバー上で起動し、呼び出し(コマンドライン、上端、ドラッグ、ショートカット)、
隠す操作、入力、終了を確認します。自分で試すには:

```sh
export TSUMIBA_HOME=$(mktemp -d) LIBGL_ALWAYS_SOFTWARE=1 XDG_SESSION_TYPE=x11
xvfb-run -a -s "-screen 0 1440x900x24" bash -c '
  tsumiba --background & sleep 4
  tsumiba show; sleep 1
  xdotool windowfocus "$(xdotool search --onlyvisible --name "^tsumiba$" | head -1)"   # 下記参照
  xdotool mousemove 300 120 click 1 type "typed by a script"
  import -window root shot.png
  tsumiba quit'
```

ウィンドウマネージャーの無い仮想ディスプレイでは、次の 2 点に注意してください。

- **誰もキーボードフォーカスを渡してくれません。**入力する前に、そのウィンドウに対して
  `xdotool windowfocus` を呼んでください(ウィンドウが表示されたあとに実行する必要があります)。
- スクリーンショットには**ウィンドウの枠も影もありません**。既定のサイズなら、1440×900 の画面で
  パネルは `x 200..1240、y 0..380` にあります。

`LIBGL_ALWAYS_SOFTWARE=1` は Mesa にソフトウェアで描画させます。GPU の無いマシンにはこれが必要です。

Windows と macOS のコードは、Linux マシンでは実行できません。CI がそこでコンパイル・lint し、スモークテスト
もします。誰かが実際に使うまでは、「確認済み」と書かないでください。

## このプロジェクトの流儀 {#house-rules}

- **ユーザーに見える文字列はすべて、英語と日本語の両方が必要です**
  (`crates/tsumiba/src/i18n.rs`)。拡張機能にも同じ規則があります(`extension/src/lib/i18n.js`)。
- 隠れたウィンドウは、何かが再描画を求めない限りフレームを受け取りません。別のスレッドからウィンドウに
  働きかけるものは、すべて `Control::send` を通してください。
- 確認の状況には正直であること。そのシステムで動かしていなければ、「動く」ではなく「実装済み」と
  書いてください。

## ドキュメント {#the-docs}

今読んでいるサイトは [mdBook](https://rust-lang.github.io/mdBook/) で作られています。

```sh
scripts/build-docs-site.sh                    # target/docs-site にビルド(トップページ、en/、ja/)
node scripts/check-docs-links.mjs             # リンク切れと、ネットワークから読み込まれるものの検査
```

2 つの本(`docs-site/en`、`docs-site/ja`)は同じページを持っていなければならず、ずれると検査が失敗します。
テーマ(`docs-site/theme`)とスクリーンショット(`docs-site/landing/screenshots`)は共有で、ビルドスクリプトが
それぞれの本へコピーするので、編集するのは元のファイルです。ビルドスクリプトを一度も実行せずに本のディレクトリで
`mdbook serve` を実行しないでください。画像とテーマがまだ無い状態になります。
