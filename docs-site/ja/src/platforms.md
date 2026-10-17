# 対応プラットフォーム {#platform-support}

何がどこで動くか。そして同じくらい大切な、どのくらい確かなのか。凡例は次のとおりです。

- <span class="tag">確認済み</span> 作者が端から端まで動かした(Linux/X11 の仮想ディスプレイ上で、スクリプトによる
  ポインターとキーボード操作を使って)。
- <span class="tag">CI</span> そのオペレーティングシステム上で、プッシュのたびにコンパイル・lint・自動テストされている。
  ロジックと保存層であって、画面上のウィンドウではない。
- <span class="tag">実装済み</span> コードはあるが、そのシステムでは誰もまだ動かしていない。
- <span class="tag">不可能</span> オペレーティングシステムが許していない。

## 一覧表 {#the-matrix}

| | Linux(X11) | Linux(Wayland) | Windows | macOS |
| --- | --- | --- | --- | --- |
| ビルド、lint、ユニットテスト | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> |
| 保存層・HTTP API・コマンドラインのテスト(実際のプログラムを動かします) | <span class="tag">確認済み</span> <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> |
| 実際のプログラムが起動し、イベントループと API が立ち上がり、`show` / `hide` / `quit` が受け付けられる | <span class="tag">確認済み</span> | <span class="tag">CI</span> | <span class="tag">CI</span> スモークテスト(参考扱い) | <span class="tag">CI</span> スモークテスト(参考扱い) |
| ウィンドウ:隠れた状態で起動し、表示・非表示、スライドで現れる | <span class="tag">確認済み</span> | 通常のウィンドウ、スライドなし。<span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| GPU の無い環境でのウィンドウ作成(ソフトウェア描画) | <span class="tag">確認済み</span>(上記のすべての検証と同じく Mesa のソフトウェアレンダラー) | <span class="tag">実装済み</span> | <span class="tag">CI</span> GPU の無いランナー(参考扱い)。wgpu へのフォールバックで、実際の仮想マシンでは未試験 | <span class="tag">実装済み</span> |
| メモ:書く、自動保存、切り替え、検索 | <span class="tag">確認済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| メモ:ピン留め、削除と元に戻す | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ファイル:タイルとサムネイル、設定画面 | <span class="tag">確認済み</span>(目視) | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ファイル:右クリックメニュー、名前変更、外すと元に戻す | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| 画面上端のトリガー(留まる、かすめる、ドラッグ) | <span class="tag">確認済み</span> | <span class="tag">不可能</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| グローバルショートカット | <span class="tag">確認済み</span> | <span class="tag">不可能</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| `tsumiba show / hide / toggle` | <span class="tag">確認済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| トレイアイコン | <span class="tag">実装済み</span>(ステータス通知ホストが必要。GNOME では AppIndicator 拡張) | <span class="tag">実装済み</span>(同左) | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ほかのプログラムから積み場へのファイルのドロップ | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ファイルを追加…、コピーを保存…(ネイティブダイアログ) | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| 開く、フォルダーで表示、ファイルをコピー | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| 日本語の表示(システムの CJK フォントへのフォールバック) | <span class="tag">確認済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| 日本語の入力(IME) | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ログイン時に起動 | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> | <span class="tag">実装済み</span> |
| ファイルを積み場の*外へ*ドラッグする | <span class="tag">未実装</span> | <span class="tag">未実装</span> | <span class="tag">未実装</span> | <span class="tag">未実装</span> |
| コード署名と公証 | なし | なし | なし | ad-hoc 署名のみ |

[ブラウザ拡張](browser-extension.md):ロジック、実際の tsumiba サーバーとのやり取り、そして**ヘッドレス Chromium** に
読み込んだビルド済みの拡張(サービスワーカー、メニュー、ショートカット、ポップアップ、オプション)は自動テスト
の対象で、Firefox 版は `web-ext lint` で検査しています。**Firefox・Edge・Brave では一度も実行していない**ので、
それらは<span class="tag">実装済み</span>にとどまります。

## 表の読み方 {#reading-the-table}

**Linux/X11 が基準です。**「確認済み」と書かれたものはすべて、そこで、スクリプトによるキーボードとポインターを
使った仮想ディスプレイ(Xvfb)上で動かしました。起動時にウィンドウが隠れたままであること、`tsumiba show`・
`toggle`・ショートカットから現れること、上端のトリガー(かすめても開かないこと、ドラッグするとすぐ開くことを
含む)、ポインターが外れたら隠れること、メモの入力と保存、検索、ファイルのタイルとサムネイル、設定画面、
日本語の表示、そして起動中のアプリに対するコマンドライン全体です。スモークテストの部分を行うスクリプトは
リポジトリにあり(`scripts/smoke-gui-linux.sh`)、CI でも実行されます。

**Windows と macOS は、ビルドとテストはされていますが、動いているところをまだ見ていません。**同じ
プログラムがそれらに向けてコンパイルされ、保存層・HTTP API・コマンドラインはそこでテストされ、CI のスモーク
テストが各環境で実際のプログラムを起動します(GPU の無いランナーでウィンドウが作れるかどうかも、ビルドを
失敗させずに報告します)。プラットフォーム固有の部分(ポインターの読み取り、トレイ、ショートカット、コンソール
への接続)は、それぞれのシステム自身のインターフェースに対して書かれています。まだ誰もしていないのは、
実際の Windows や macOS のマシンの前に座って積み場を使うことです。粗いところがあると思ってください。
[報告](https://github.com/hjosugi/tsumiba/issues)をお待ちしています。

**Wayland は、バグではなく意図された制約です。**Wayland の設計は、アプリケーションが自分のウィンドウの外の
ポインターを読むこと、キーをグローバルに捕まえること、自分でウィンドウの位置を決めて前面に出すことを
禁じています。そのため、上端トリガーとショートカットは存在し得ません。積み場は通常のウィンドウで、
`tsumiba toggle` を自分でキーに割り当てます([積み場を呼び出す](summoning.md#wayland))。

**ドラッグで取り出す機能は、どこにも実装されていません。**理由はウィンドウツールキットにあります
([ファイル](files.md#dragging-files-out))。

## 予定していないもの {#not-planned}

設計上、tsumiba にはクリップボード履歴、クラウド同期、アカウントはありません。範囲はメモとファイルです。
