# HTTP API {#http-api}

アプリが動いている間(または `tsumiba serve` の間)、tsumiba は同じコンピューター上で小さな HTTP API に
応答します。[コマンドライン](cli.md)と[ブラウザ拡張](browser-extension.md)がその 2 つのクライアントで、
3 つ目のクライアントはあなたが作れます。

- **アドレス:**`http://127.0.0.1:<port>/v1`、既定のポートは **47821**(`api.port`)。`127.0.0.1` のみで、
  ネットワークから到達できることはありません。
- **形式:**JSON の入出力、UTF-8。エラーは `{"error": "メッセージ"}` で、種類は HTTP ステータスで分かります。
- **認証:**`GET /ping` 以外のすべてに `Authorization: Bearer <token>` が必要です。トークンは `config/token`
  にあり、`tsumiba token` で表示されます。
- **バージョン:**パスはすべて `/v1` で始まります。フィールドは増えることはあっても減りません。知らない
  フィールドは無視してください。

```sh
TOKEN=$(tsumiba token)
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:47821/v1/status
# {"notes":3,"files":7,"visible":false,"max_upload_mb":512}
```

tsumiba の起動時に既定のポートが使用中だった場合は、別のポートが選ばれ、`config/server.json`
(`{"port": …, "pid": …, "version": …}`)に記録されます。コマンドラインはこのファイルを読みますが、
ほかのクライアントにはポートを教える必要があります。

## セキュリティモデル {#security-model}

訪れたどんな Web ページも、ブラウザに `127.0.0.1` へのリクエストを送らせることができます。そのため、
ループバックで待ち受けているだけでは保護になりません。すべてのリクエストは、何かを読んだり実行したり
する前に、次の 3 つの検査をこの順で通ります。

1. **`Host`** は `127.0.0.1:<port>`・`localhost:<port>`・`[::1]:<port>` のいずれかでなければなりません。
   それ以外は `403 unexpected Host header` です。これは、攻撃者のドメインが `127.0.0.1` に解決されるよう
   仕向けられる *DNS リバインディング*を防ぎます。
2. **`Origin`** は、あるときには、**ブラウザ拡張**のオリジンでなければなりません。
   `chrome-extension://…`、`moz-extension://…`、`safari-web-extension://…` です。通常の Web ページの
   オリジン(`https://example.com`、`http://localhost`、`null`)は、有効なトークンがあっても
   `403 this origin is not allowed` です。`Origin` の無いリクエスト(curl やスクリプト)はこの検査を通ります。
3. **トークン。**`Authorization: Bearer <16 進数 64 桁>` を、一定時間で比較します。無い、または違う場合は
   `401 missing or wrong token` です。トークンが Web ページに渡ることはなく、拡張機能は自分のストレージに
   保管します。

そのほか:

- 認証不要のエンドポイントは `GET /v1/ping` だけで、これが明かすのは、これが tsumiba であることと
  そのバージョンだけです。クライアントが、同じポートを握っている別のプログラムと区別できるようにするため
  です。それでも `Host` と `Origin` の検査は通る必要があります。
- **CORS:**許可された拡張のオリジンには、`Access-Control-Allow-Origin: <そのオリジン>` と `Vary: Origin` が
  付きます。許可されたオリジンからのプリフライト(`OPTIONS`)には、`204` と
  `Access-Control-Allow-Methods: GET, POST, PATCH, DELETE, OPTIONS`、
  `Access-Control-Allow-Headers: authorization, content-type, x-tsumiba-name, x-tsumiba-source`、
  `Access-Control-Max-Age: 600` が返ります。ブラウザはプリフライトにトークンを付けないため、トークンは不要
  です。それ以外のオリジンからのプリフライトは `403` で、CORS ヘッダーは一切付きません。
- レスポンスには `Cache-Control: no-store` と `X-Content-Type-Options: nosniff` が付きます。
- **サイズ:**JSON ボディは約 4 MiB まで(それを超えると `413`)、アップロードは `storage.max_upload_mb`
  (既定 512 MiB)までです。上限は `Content-Length` で事前に調べ、さらにボディが流れ込む間にも調べます。
  長さを偽るクライアントや、長さを送らない(チャンク転送)クライアントも上限で打ち切られ、ディスクには
  何も残りません。
- ファイル名はサニタイズされます。最後のパス要素だけが残り、Windows で禁止されている文字は `_` になり、
  `CON` のような予約名には接頭辞が付き、長さには上限があります。`../../etc/cron.d/x` のような名前は、
  積み場の中の `x` になります。
- URL の中の ID は、小文字の 16 進数ちょうど 20 桁でなければなりません。それ以外はファイルシステムに
  到達できません。

## データの形 {#data-shapes}

**メモ**

```json
{
  "id": "01a148d44c055be0439c",
  "title": "Release checklist",
  "body": "# Release checklist\n- [ ] tag it\n",
  "pinned": true,
  "created_ms": 1760719443000,
  "updated_ms": 1760719443000
}
```

`title` は、先頭の `#` を除いた最初の空でない行です。時刻は Unix エポックからのミリ秒です。
`created_ms` は ID の一部で、`updated_ms` はファイルの更新日時です。

**ファイル**

```json
{
  "id": "01a148d44c5b4e5d2364",
  "name": "screenshot.png",
  "size": 2032,
  "added_ms": 1760719443000,
  "pinned": false,
  "source": "https://example.com/page",
  "link": false,
  "path": "/home/me/.local/share/tsumiba/files/01a148d44c5b4e5d2364/screenshot.png"
}
```

`source` は、分かっていれば取得元(ブラウザからのアップロードならページのアドレス、ローカルからの
取り込みなら元のパス)で、`null` のこともあります。`link` は、場所だけを覚えている場合に真になり、
そのとき `path` は元のファイルを指します。フォルダーのリンクの `size` は `0` です。

## エンドポイント {#endpoints}

| メソッドとパス | 認証 | 動作 |
| --- | --- | --- |
| `GET /v1/ping` | 不要 | `{"app":"tsumiba","version":"0.1.0"}` |
| `GET /v1/status` | 必要 | `{"notes": n, "files": n, "visible": bool, "max_upload_mb": n}` |
| `POST /v1/window` | 必要 | 積み場を表示・非表示・切り替え。`204` |
| `POST /v1/quit` | 必要 | tsumiba を終了する。`204` |
| `GET /v1/notes` | 必要 | メモの一覧(ピン留め優先、そのあとは新しい順) |
| `POST /v1/notes` | 必要 | メモを作る。`201` |
| `GET /v1/notes/{id}` | 必要 | メモ 1 件。無ければ `404` |
| `PATCH /v1/notes/{id}` | 必要 | メモの本文やピン留めを変える。`200` |
| `DELETE /v1/notes/{id}` | 必要 | メモを削除する(ゴミ箱へ)。`204` |
| `GET /v1/files` | 必要 | ファイルの一覧(ピン留め優先、そのあとは新しい順) |
| `POST /v1/files` | 必要 | ファイルをアップロードする(生のボディ)。`201` |
| `POST /v1/files/import` | 必要 | このマシン上のパスからファイルを取り込む |
| `DELETE /v1/files/{id}` | 必要 | ファイルを外す(コピーはゴミ箱へ、リンクは忘れるだけ)。`204` |
| `OPTIONS *` | 不要 | 拡張オリジン向けの CORS プリフライト。`204` |

### `GET /v1/status` {#get-v1status}

`notes` と `files` は件数、`visible` は積み場がいま画面に出ているかどうか(`tsumiba serve` では常に
`false`)、`max_upload_mb` はサーバーが受け付ける最大のアップロードサイズ(MiB。`storage.max_upload_mb`、
既定 512)です。ファイルをアップロードするクライアントは、先にこれを読んで、大きすぎるファイルを自分で
断るのが望ましいです。サーバーはアップロードを早い段階で断るとき、`413` を返して、ボディがまだ送られて
いる途中で接続を閉じます。システムによっては、クライアントには `413` ではなくネットワークエラーとして
見えます。ブラウザ拡張はまさにそうしています。

### `POST /v1/window` {#post-v1window}

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"action":"toggle"}' http://127.0.0.1:47821/v1/window
```

`action` は `"show"`・`"hide"`・`"toggle"` のいずれかで、それ以外は `400` です。

### メモ {#notes}

```sh
# 一覧。q で絞り込み(本文に対して大文字小文字を区別せず一致)、limit で件数を制限
curl -s -H "Authorization: Bearer $TOKEN" 'http://127.0.0.1:47821/v1/notes?q=release&limit=10'

# 作成
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"text":"# Hello\nfrom curl","pinned":true}' http://127.0.0.1:47821/v1/notes

# 変更:どちらのフィールドも省略可能。省略したものは変わらない
curl -s -X PATCH -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"text":"new text","pinned":false}' http://127.0.0.1:47821/v1/notes/01a148d44c055be0439c
```

`POST` は `{"text": 文字列, "pinned": 真偽値?}` を受け取ります。`text` は必須で、4 MiB まで入れられます。
`PATCH` は `{"text": 文字列?, "pinned": 真偽値?}` です。

### ファイル {#files}

**アップロード**では、ファイルのバイト列をリクエストボディにします。名前と取得元はヘッダーに、UTF-8 の
パーセントエンコードで入れます(`A-Z a-z 0-9 . _ ~ -` 以外はすべて `%XX` になります)。

| ヘッダー | 意味 |
| --- | --- |
| `X-Tsumiba-Name` | ファイル名。既定は `file` |
| `X-Tsumiba-Source` | 取得元。通常はページのアドレス。省略可 |
| `Content-Type` | 名前に拡張子が無いときに拡張子を補うためだけに使われます(`image/png` → `.png`。ほかに jpeg・gif・webp・svg・avif・pdf・txt・html・json・zip) |

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H 'X-Tsumiba-Name: report%202026.pdf' \
  -H 'X-Tsumiba-Source: https%3A%2F%2Fexample.com%2Freports' \
  -H 'Content-Type: application/pdf' \
  --data-binary @report.pdf http://127.0.0.1:47821/v1/files
```

大きすぎると `413` です。

**取り込み**は、tsumiba が動いているマシン上にすでにあるファイルを読むよう頼むもので、バイト列を HTTP で
送らずに済みます(`tsumiba add` が使っているのはこれです)。

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"paths":["/home/me/a.txt","/home/me/photos"],"link":false}' \
  http://127.0.0.1:47821/v1/files/import
# {"added":[{...}],"failed":[{"path":"...","error":"..."}]}
```

パスは絶対パスでなければなりません(相対パスは `failed` に報告されます)。`link: true` はコピーの代わりに
リンクを保存し、省略すると `storage.mode` に従います。フォルダーは常にリンクです。一部のパスが失敗して
いてもステータスは `200` なので、`failed` を確認してください。

### エラー {#errors}

| ステータス | 条件 |
| --- | --- |
| `400` | 不正な JSON、未知の `action`、不正な ID |
| `401` | トークンが無い、または違う |
| `403` | `Host` が違う、または `Origin` がブラウザ拡張ではない |
| `404` | そのメモやファイルが無い、またはそのルートが無い |
| `413` | ボディが上限を超えている |
| `500` | ディスク上で何かが失敗した。メッセージに内容が出る |

## 互換性 {#compatibility}

API のバージョンはパスで表されます。`/v1` の中では追加だけが行われる、というのが方針です。新しい
エンドポイントや新しいレスポンスのフィールドが増えます。クライアントは、知らないフィールドを無視して
ください。
