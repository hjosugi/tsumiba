# 設定 {#configuration}

設定は `config.toml` という 1 つのファイルにあります。ほとんどはアプリ内(**設定**、歯車ボタン、または
<kbd>Ctrl</kbd>/<kbd>⌘</kbd>+<kbd>,</kbd>)で変更でき、すべてをファイルの編集で変更できます。
ファイルの場所は `tsumiba paths` で表示できます。

- **ファイルが無い、キーが無い:**既定値になります。完全なファイルを書く必要はありません。
- **未知のキー:**無視されるので、新しいバージョンのファイルでも読み込めます。
- **範囲外の数値:**妥当な範囲に丸められます。
- **読み込まれるとき:**起動時です。手で編集した内容は、tsumiba を再起動すると反映されます
  (`tsumiba quit` のあと `tsumiba`)。
- **書き込まれるとき:**アプリ内で設定を変更したときです。アプリはファイル全体を書き直すので、手で足した
  コメントは失われますが、値は保たれます。
- **壊れたファイル**(TOML として不正な誤字や、`mode = "teleport"` のような型の誤り)は、起動時に報告され、
  tsumiba はそのセッションを既定値で動かします(起動を拒否はしません)。壊れたファイル自体は、アプリで
  設定を変更するまでそのまま残り、変更すると新しいファイルが書かれます。**設定画面を触る前に、誤字を
  直してください。**

## リファレンス {#reference}

### `[general]` {#general}

| キー | 型 | 既定値 | 意味 |
| --- | --- | --- | --- |
| `language` | `"auto"`・`"en"`・`"ja"` | `"auto"` | 表示言語。`auto` はシステムのロケールに従い、`ja…` なら日本語、それ以外は英語 |
| `theme` | `"system"`・`"light"`・`"dark"` | `"system"` | 配色テーマ。`system` は OS に従う |
| `launch_at_login` | 真偽値 | `false` | ログイン時に tsumiba を(トレイで、`--background` で)起動する。設定画面から変更してください。ログイン項目の作成・削除も同時に行われます |
| `renderer` | `"auto"`・`"glow"`・`"wgpu"` | `"auto"` | ウィンドウの描画方法(下記)。環境変数 `TSUMIBA_RENDERER` が優先される |

**レンダラー。**`auto` はどこでも OpenGL(`glow`)を使います。**Windows に限り**、OpenGL でウィンドウを作れない場合、
tsumiba は自分自身を一度だけ **wgpu** レンダラー(DirectX 12。最後の手段として Microsoft のソフトウェア
ラスタライザー WARP)で再起動します。これは、グラフィックドライバーが無く、Windows 組み込みの OpenGL 1.1 の
スタブしか無い仮想マシンやリモートデスクトップのセッションで起きます。そのときログには
`egui_glow requires opengl 2.0+` のような文言が出ます。`glow` は OpenGL を強制し(失敗したらエラーを表示します)、
`wgpu` は wgpu を強制します。Linux と macOS のビルドには wgpu が含まれないため、そこでの `wgpu` は OpenGL に
フォールバックします。`glow` を強制した場合は再試行せず、再試行自体がループすることもありません。

> [!NOTE]
> このフォールバックを実際に動かしているのは、CI の GPU の無い Windows ランナー(参考扱いのジョブ)だけで、
> 実際の仮想マシンでは作者は試していません。

### `[panel]` {#panel}

| キー | 型 | 既定値 | 範囲 | 意味 |
| --- | --- | --- | --- | --- |
| `show_notes` | 真偽値 | `true` | | メモの列を表示する |
| `show_files` | 真偽値 | `true` | | ファイルの列を表示する |
| `width` | 数値(ポイント) | `1040` | 480〜3000 | パネルの幅。モニターより広くなることはない |
| `height` | 数値(ポイント) | `380` | 240〜2000 | パネルの高さ。モニターより高くなることはない |
| `auto_hide` | 真偽値 | `true` | | ポインターが外れたら隠す([積み場を呼び出す](summoning.md#hiding)) |

`show_notes` と `show_files` の両方が `false` の場合は、両方がオンに戻されます。空のパネルを望む人は
いないからです。

### `[trigger]` {#trigger}

| キー | 型 | 既定値 | 範囲 | 意味 |
| --- | --- | --- | --- | --- |
| `hotkey` | 文字列 | `"Ctrl+Alt+Space"` | | 積み場の表示を切り替えるグローバルショートカット。空(`""`)でオフ |
| `edge` | 真偽値 | `true` | | ポインターが上端に留まったら開く |
| `edge_dwell_ms` | 整数(ms) | `250` | 0〜5000 | 留まる必要がある時間。`0` で即座に開く。ファイルをドラッグしているときは無視される |

**ショートカットの書き方。** 1 つ以上の修飾キーのあとに 1 つのキーを `+` でつなげて書きます。大文字小文字は
問いません。修飾キーは `Ctrl`/`Control`、`Alt`/`Option`、`Shift`、`Super`/`Cmd`/`Meta`、そして `CmdOrCtrl`
(macOS では <kbd>⌘</kbd>、それ以外では <kbd>Ctrl</kbd>)です。キーは、英字、数字、`F1`〜`F24`、`Space`、`Enter`、
`Tab`、`Up`/`Down`/`Left`/`Right`、`Home`、`End`、`PageUp`、`PageDown`、`Insert`、`Delete`、`Backspace`、
そして `-`・`=`・`,`・`.`・`/`・`;`・`` ` `` などの記号です。ファンクションキーを除き、修飾キーが少なくとも
1 つ必要です。例:`"Ctrl+Alt+Space"`、`"Ctrl+Shift+K"`、`"Alt+F1"`、`"Cmd+Option+Space"`、
`"CmdOrCtrl+Shift+Y"`。Wayland には登録できるグローバルショートカットが無く、起動時に tsumiba がそう伝えます。

### `[storage]` {#storage}

| キー | 型 | 既定値 | 範囲 | 意味 |
| --- | --- | --- | --- | --- |
| `mode` | `"copy"`・`"link"` | `"copy"` | | 追加したファイルをどうするか:専用の**コピー**を保存するか、場所だけを覚える(**リンク**)か。フォルダーは常にリンク |
| `max_upload_mb` | 整数(MiB) | `512` | 1〜65536 | [HTTP API](api.md) が受け付ける最大のファイルサイズ(たとえばブラウザ拡張から) |

### `[api]` {#api}

| キー | 型 | 既定値 | 意味 |
| --- | --- | --- | --- |
| `enabled` | 真偽値 | `true` | コマンドラインとブラウザ拡張が使うローカル HTTP API を動かす。オフにすると、拡張は接続できず、コマンドラインはファイルを直接扱う |
| `port` | 整数 | `47821` | ループバックのポート。`0` ならシステムが空きポートを選ぶ。`0` か 1024〜65535 を使ってください。ポートが使用中なら、tsumiba は空きポートを選んで通知し、コマンドライン向けに記録します(その場合、拡張には新しいポートを教える必要があります) |

## 注釈つきの例 {#an-annotated-example}

以下の値はすべて既定値なので、このファイルは何も変えません。不要なところは削ってください。

```toml
[general]
language = "auto"          # "auto" | "en" | "ja"
theme = "system"           # "system" | "light" | "dark"
launch_at_login = false
renderer = "auto"          # "auto" | "glow" | "wgpu"(wgpu は Windows のみ)

[panel]
show_notes = true
show_files = true
width = 1040.0             # ポイント。480-3000
height = 380.0             # ポイント。240-2000
auto_hide = true           # ポインターが外れたら隠す

[trigger]
hotkey = "Ctrl+Alt+Space"  # "" でショートカットをオフ
edge = true                # ポインターが上端に留まったら開く
edge_dwell_ms = 250        # 0-5000。ファイルのドラッグ中は待たない

[storage]
mode = "copy"              # "copy" | "link"
max_upload_mb = 512        # 1-65536

[api]
enabled = true
port = 47821               # 0 = 空いているポートを自動選択
```

## 環境変数 {#environment-variables}

| 変数 | 効果 |
| --- | --- |
| `TSUMIBA_HOME` | すべてをこのフォルダーの下に置く(中の `data/` と `config/`) |
| `TSUMIBA_DATA_DIR` | データフォルダーだけ。`TSUMIBA_HOME` より優先 |
| `TSUMIBA_CONFIG_DIR` | 設定フォルダーだけ。`TSUMIBA_HOME` より優先 |
| `TSUMIBA_LOG` | tsumiba 自身のメッセージのログレベル:`error`・`warn`・`info`(既定)・`debug`・`trace`・`off` |
| `TSUMIBA_RENDERER` | `auto`・`glow`・`wgpu`:`general.renderer`([上記](#general))を上書きする |

`tsumiba` と `tsumiba quit` の 2 つのコマンドは、同じ環境変数を見ている必要があります。お互いのファイルを
見つける方法がそれだからです。

## ファイルの場所 {#where-things-are}

| | Linux | macOS | Windows |
| --- | --- | --- | --- |
| **データ**(メモ、ファイル、ゴミ箱、ログ) | `$XDG_DATA_HOME/tsumiba`(通常は `~/.local/share/tsumiba`) | `~/Library/Application Support/io.github.hjosugi.tsumiba` | `%APPDATA%\hjosugi\tsumiba\data` |
| **設定**(`config.toml`、トークン) | `$XDG_CONFIG_HOME/tsumiba`(通常は `~/.config/tsumiba`) | データと同じフォルダー | `%APPDATA%\hjosugi\tsumiba\config` |

中身は次のとおりです。

| ファイル | 場所 | 内容 |
| --- | --- | --- |
| `config.toml` | 設定 | このページの内容 |
| `token` | 設定 | ペアリング用のトークン(Linux と macOS ではあなただけが読める) |
| `server.json` | 設定 | tsumiba の実行中に書かれる、ポートとプロセス ID。コマンドラインが見つけるためのもの。終了時に削除され、クラッシュで残ったものは無視される |
| `notes/<id>.md` | データ | メモ 1 つにつき 1 ファイル |
| `files/<id>/<name>` | データ | 保存したファイル 1 つにつき 1 フォルダー |
| `index.json` | データ | ピン留めとファイルのメタデータ。壊れていれば再構築される |
| `trash/` | データ | 削除した項目。30 日間保持 |
| `tsumiba.log`、`tsumiba.log.old` | データ | ログ。起動時に 1 MiB を超えていればローテーションされる |

データフォルダーの詳細は[データと同期](data-and-sync.md)を参照してください。
