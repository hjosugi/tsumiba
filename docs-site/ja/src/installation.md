# インストール {#installation}

リリースは[リリースページ](https://github.com/hjosugi/tsumiba/releases/latest)にあり、
`SHA256SUMS.txt` が添えられています。下のインストーラースクリプトは、お使いの環境に合うアーカイブを
ダウンロードし、何かをインストールする前にこのチェックサムと照合します。

> [!NOTE]
> リリースはまだコード署名されていません。初回起動時に、Windows の SmartScreen と macOS の Gatekeeper が
> それぞれ確認を求めます。手順は以下にあります。

## Windows {#windows}

```powershell
irm https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.ps1 | iex
```

`%LOCALAPPDATA%\Programs\tsumiba` に展開し、そのフォルダーをユーザーの `PATH` に追加して、スタートメニューに
ショートカットを作ります。管理者権限は不要です。

手動で入れる場合は、`tsumiba-<version>-windows-x86_64.zip` をダウンロードして好きな場所に展開し、
`tsumiba.exe` を実行します。ほかに何かをインストールする必要はなく、tsumiba 自身のフォルダー以外には
何も書き込みません([データと同期](data-and-sync.md))。

初回は SmartScreen が「Windows によって PC が保護されました」と表示します。**詳細情報**を選び、
**実行**を押してください。

## macOS {#macos}

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

`Tsumiba.app` を `~/Applications` に入れ、`tsumiba` コマンドを `~/.local/bin` に置きます。手動で入れる場合は、
`.dmg` を開いて **Tsumiba** を「アプリケーション」にドラッグするか、
`tsumiba-<version>-macos-universal.zip` を展開してください。Apple シリコンと Intel の両方を、ひとつの
ユニバーサルビルドでカバーします。

アプリは ad-hoc 署名のみで**公証(notarization)されていない**ため、Gatekeeper が初回起動をブロックします。
アプリを右クリックして**開く**を選ぶか、隔離属性を削除してください。

```sh
xattr -dr com.apple.quarantine /Applications/Tsumiba.app
```

tsumiba は「エージェント」アプリです。Dock アイコンも専用のメニューバーもありません。メニューバーの
ステータス領域にアイコンが出て、画面の上端からスライドして現れます。

## Linux {#linux}

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

`~/.local` に(root なしで)`tsumiba` 本体、デスクトップファイル、アイコンをインストールします。あるいは、
リリースページの `tsumiba-<version>-linux-<arch>.tar.gz` を展開して、中の `install.sh` を実行します。

```sh
./install.sh                      # ~/.local へ
./install.sh --prefix /usr/local  # システム全体へ(sudo で実行)
```

どのアーキテクチャ向けがビルドされたかは、リリースページに書かれています。`~/.local/bin` が `PATH` に
入っていることを確認してください。

### Linux の動作要件 {#linux-requirements}

tsumiba は OpenGL で描画し、X11 または Wayland と通信します。これらのライブラリは起動時に読み込まれます。
通常のデスクトップならすべて入っていますが、最小構成のコンテナやサーバーイメージには無いことがあります。
ライブラリが足りないときは、tsumiba がどれが足りないかを表示して終了します。必要なものは次のとおりです。

- OpenGL(`libGL`、`libEGL`):Mesa または GPU ベンダーのドライバー
- **X11 セッション:**`libX11`、`libxcb`、`libxkbcommon-x11`。X11 のウィンドウは最後のもの
  (Debian と Ubuntu では `libxkbcommon-x11-0`)を実行時に必要とします。無い場合、tsumiba はパッケージ名を
  添えたヒントを表示して終了します
- **Wayland セッション:**`libxkbcommon` と `libwayland-client`

```sh
# Debian、Ubuntu(一般的なパッケージ名)
sudo apt-get install libgl1 libegl1 libxkbcommon0 libxkbcommon-x11-0 libx11-6 libxcb1 libwayland-client0
# Fedora
sudo dnf install mesa-libGL mesa-libEGL libxkbcommon libxkbcommon-x11 libX11 libxcb libwayland-client
# Arch
sudo pacman -S mesa libxkbcommon libxkbcommon-x11 libx11 libxcb wayland
```

GPU ドライバーの無いマシン(素の VM やコンテナ)では、Mesa のソフトウェアレンダラーが使えます。
`LIBGL_ALWAYS_SOFTWARE=1` を設定してください。作者のヘッドレス検証もこの方法で動いています。

> [!NOTE]
> **Wayland:** Wayland ではアプリケーションがポインターを監視することもキーを捕まえることもできないため、
> 画面上端のトリガーとグローバルショートカットはありません。デスクトップのキーボード設定で、
> `tsumiba toggle` コマンドをキーに割り当ててください。[積み場を呼び出す](summoning.md#wayland)を参照。

### Nix {#nix}

```sh
nix run github:hjosugi/tsumiba
```

リポジトリの flake には開発用シェルもあります:`nix develop`。

## ソースからビルドする {#build-from-source}

Rust 1.92 以降が必要です。

```sh
git clone https://github.com/hjosugi/tsumiba
cd tsumiba
cargo build --release          # target/release/tsumiba
cargo test --workspace
```

Linux では、`pkg-config` と、`libxcb`・`libxkbcommon`・`libx11`・`wayland` の開発用ファイルも必要です。
Debian と Ubuntu では次のとおりです。

```sh
sudo apt-get install pkg-config libxcb1-dev libxcb-randr0-dev libxkbcommon-dev \
  libx11-dev libwayland-dev
```

`nix develop` なら、これらすべてに加えて、ヘッドレス GUI 検証用のツールも揃います
([開発に参加する](contributing.md))。

## 更新と削除 {#updating-and-removing}

**更新:** インストーラーをもう一度実行するか、ファイルを手で置き換えてください。メモとファイルは別の場所に
あり、影響を受けません。先に古いほうを終了してください(`tsumiba quit`)。

**削除:** tsumiba を終了してプログラムを削除します。すべて消したい場合は、データフォルダーと設定フォルダー
も削除してください(`tsumiba paths` で場所を表示できます。[データと同期](data-and-sync.md)を参照)。
「ログイン時に起動」をオンにしていた場合は、先に設定でオフにするか、システムが持つ自動起動の登録を
削除してください。
