# コマンドライン {#command-line}

アプリとコマンドラインは、同じひとつのプログラムです。コマンド無しの `tsumiba` は積み場を開きます。
コマンドを付けると、その 1 つの処理だけを行って終了します。

```text
tsumiba [--background] [COMMAND]
```

積み場を変更するコマンド(`note`・`add`・`show`・`hide`・`toggle`・`quit`)は、**起動中のアプリ**があればそれと
通信するので、ウィンドウは最新の状態を保ち、ファイルを扱うプロセスも 1 つで済みます。起動中のアプリが
無いときは、`note`・`add`・`list` は**ファイルを直接**扱います。画面が無いスクリプトや cron、SSH でも
安全に使えます。

## コマンド一覧 {#commands}

| コマンド | 動作 |
| --- | --- |
| `tsumiba` または `tsumiba run` | 積み場を開く。起動していなければ起動する |
| `tsumiba --background` | 積み場を開かずにトレイで起動する(「ログイン時に起動」が実行するのはこれ) |
| `tsumiba show` | 積み場を表示する(必要なら起動する) |
| `tsumiba hide` | 積み場を隠す |
| `tsumiba toggle` | 隠れていれば表示、表示中なら隠す(必要なら起動する)。**Wayland ではこれをキーに割り当ててください。** |
| `tsumiba note [TEXT]… [--pin]` | メモを追加する |
| `tsumiba add PATH… [--link]` | ファイルやフォルダーを追加する |
| `tsumiba list [--notes] [--files] [-q TEXT] [--json]` | メモとファイルを一覧表示する |
| `tsumiba quit` | 起動中の tsumiba を終了する |
| `tsumiba token [--regenerate]` | ブラウザ拡張のペアリング用トークンを表示する |
| `tsumiba paths` | tsumiba がファイルを置いている場所を表示する |
| `tsumiba serve [--port N]` | ウィンドウ無しでローカル API を実行する |
| `tsumiba --version`、`tsumiba --help` | バージョンとヘルプ |

`hide` と `quit` は、tsumiba が起動していないとやることが無いので、そう伝えます。`show` と `toggle` は
代わりに tsumiba を起動します。起動することが、そのまま表示することだからです。

### `note` {#note}

```sh
tsumiba note 牛乳を買う                      # 引数は空白でつながれる
tsumiba note $'# 打ち合わせ\n議題: ...'       # bash/zsh: $'...' で \n が改行になる
echo "傘を忘れない" | tsumiba note           # 引数なし: 標準入力から読む
tsumiba note - < draft.md                   # "-" も標準入力の意味
tsumiba note --pin "いまやっていること"
```

新しいメモの ID を表示します。空のメモは受け付けません。

### `add` {#add}

```sh
tsumiba add report.pdf                      # 積み場へコピーする
tsumiba add --link big.iso photos/          # コピーせず、場所だけを覚える
```

パスは相対パスでもかまいません。ファイルごとに `ID<TAB>名前` の行を表示します。追加できなかったもの
(存在しない、読めない)があれば、どれかを伝え、残りは追加し、失敗として終了します。フォルダーは
常にリンクになります。アプリが起動していれば、コピーはアプリが行います。`--link` を付けない限り、
コピーかリンクかは `storage.mode` で決まります。

### `list` {#list}

```sh
tsumiba list                                # メモとファイル(タブ区切り)
tsumiba list --notes -q "リリース"          # 「リリース」を含むメモだけ
tsumiba list --files --json | jq -r '.files[].path'
```

テキスト出力は 1 項目 1 行です。

```text
note	01a148d44c055be0439c	* Release checklist
file	01a148d44c5b4e5d2364	screenshot.png	2032
```

`note`、ID、タイトル(ピン留めされたメモには先頭に `* ` が付きます)、そして `file`、ID、名前、バイト単位のサイズです。`--json` を
付けると、`{"notes": [...], "files": [...]}` という 1 つのオブジェクトになり、形は
[HTTP API](api.md#data-shapes) と同じです。`-q` が絞り込むのは**メモ**(本文に一致)だけで、ファイルは
絞り込みません。

### `token` {#token}

[ブラウザ拡張](browser-extension.md)が必要とする、16 進数 64 桁のトークンを表示します。`--regenerate` は
トークンを置き換えるので、ペアリング済みのすべてのブラウザがサインアウトされます。起動中のアプリは、
再起動するまで古いトークンのままです(設定の**再生成**を使えば、サーバーを自動で再起動します)。

### `paths` {#paths}

```text
$ tsumiba paths
data    /home/me/.local/share/tsumiba
config  /home/me/.config/tsumiba/config.toml
token   /home/me/.config/tsumiba/token
log     /home/me/.local/share/tsumiba/tsumiba.log
```

### `serve` {#serve}

ウィンドウ無しで HTTP API を動かし、`tsumiba quit` を受けるか(あるいは強制終了されるまで)動き続けます。
`listening on 127.0.0.1:PORT` と表示します。すでに tsumiba が起動していると、起動を拒否します。
プロジェクト自身のテストが使っている方法で、サーバー上でも便利ですが、サーバー上の積み場も同じ普通の
ファイルです。

## 終了コード {#exit-codes}

| コード | 意味 |
| --- | --- |
| `0` | 成功 |
| `1` | 失敗。メッセージは標準エラー出力に出ます(起動していない、保存する内容が無い、ファイルを追加できなかった、など) |
| `2` | コマンドラインそのものが誤っていた(不明なコマンドやフラグ) |

## 使用例 {#examples}

**Wayland でキーに割り当てる。** GNOME では、*設定 → キーボード → キーボードショートカット → カスタム*で、
コマンドに `tsumiba toggle`。Sway では `bindsym $mod+space exec tsumiba toggle`。Hyprland では
`bind = SUPER, space, exec, tsumiba toggle`。

**どこからでも取り込む。**

```sh
# 直前のコマンド履歴を、メモとして
history | tail -20 | tsumiba note

# スクリーンショットを、そのまま積み場へ(Linux)
grim -g "$(slurp)" /tmp/shot.png && tsumiba add /tmp/shot.png
```

**SSH 越しに。** サーバーには何も表示されず、表示する必要もありません。

```sh
ssh devbox 'tsumiba note "デプロイ完了"'
```

メモは、そのマシンの積み場に入ります。複数の積み場をつなげたいときは[データと同期](data-and-sync.md)
を参照してください。

**シェル関数にする。**

```sh
n() { tsumiba note "$*"; }     # n 歯医者に電話
```

## Windows の場合 {#windows}

Windows 版は、ログイン時にコンソールウィンドウが一瞬開かないよう、GUI プログラムとして作られています。
**ターミナルから引数付きで**実行すると、そのターミナルに接続するので、`tsumiba --help` や `tsumiba list`
は出力されます。Windows は GUI プログラムの終了をターミナルに待たせないため、`cmd.exe` では出力が
終わる前にプロンプトが戻ることがあります。PowerShell では通常問題なく、`tsumiba list | more` や
`start /wait tsumiba list` なら cmd でも待たせられます。この経路は、まだ実際の Windows マシンでは
試していません。

## 環境変数 {#environment-variables}

| 変数 | 効果 |
| --- | --- |
| `TSUMIBA_HOME` | すべてをこのフォルダーの下に置く(中の `data/` と `config/`) |
| `TSUMIBA_DATA_DIR` | データフォルダーだけを指定する(`TSUMIBA_HOME` より優先) |
| `TSUMIBA_CONFIG_DIR` | 設定フォルダーだけを指定する(`TSUMIBA_HOME` より優先) |
| `TSUMIBA_LOG` | tsumiba 自身のメッセージのログレベル:`error`・`warn`・`info`(既定)・`debug`・`trace`・`off` |
| `TSUMIBA_RENDERER` | `auto`・`glow`・`wgpu`:ウィンドウの描画方法。`general.renderer`([設定](configuration.md#general))を上書きする |

`TSUMIBA_HOME` は、独立した 2 つ目の積み場や、試験用の使い捨ての積み場を動かす方法でもあります。
