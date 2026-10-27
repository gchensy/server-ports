<div align="center">

# Server Ports

**SSH 経由でリモートサーバーのポートをローカルマシンにマッピングします。**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | 日本語 | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **ステータス：開発初期段階。** 正式リリースはまだありません。トンネルエンジンとインターフェースはできており、CI が Linux・macOS・Windows でビルドとテストを行ってインストーラーを作りますが、アプリ自体はこれまでに Linux でしか実行しておらず、macOS と Windows のインストーラーは未検証です。

Server Ports は、SSH のポートフォワーディングを維持し続けてくれる小さなクロスプラットフォームのデスクトップアプリ（Windows、macOS、Linux）です。サーバーを選び、必要なポートを並べるだけで、それらが `localhost` に現れます。`ssh -L` と同じことを、ターミナルを開いてコマンドを打つ手間なしで行えます。

## 機能

- **サーバープロファイル**：SSH サーバーごとに 1 つのプロファイルを作成し、必要な数だけポートのマッピングを追加できます。
- **SSH 設定からのインポート**：「サーバーを追加」ダイアログまたは最初の画面から、`~/.ssh/config` にあるサーバーを選んでまとめて Server Ports にコピーできます。ホスト名、ユーザー、ポート、鍵ファイル（`IdentityFile`）、ポート転送（`LocalForward`）が引き継がれ、`Host *` ブロックや `Include` したファイルから継承される設定も反映されます。コピーは一度きりで、SSH 設定ファイルは読み取るだけで変更されません。
- **3 種類のログイン方法**：パスワード、秘密鍵ファイル（パスフレーズは任意）、ssh-agent（OpenSSH agent または Pageant）。
- **秘密情報は設定ファイルに書き込まない**：パスワードと鍵のパスフレーズは、あなたが希望した場合にのみ macOS のキーチェーン、Windows の資格情報マネージャー、Linux の Secret Service に保存されます。それ以外は、サーバーに接続している間だけメモリ上に保持されます。
- **ホスト鍵の検証**：初めて見るサーバーの鍵はフィンガープリントとともに表示され、あなたが承認した場合にのみ信頼されます。鍵が変わった場合は警告されます。
- **デフォルトは自分のマシンだけ**：フォワードしたポートは `127.0.0.1` で待ち受けるため、自分のマシンからしかアクセスできません。
- **自動再接続**：切断された接続は、間隔を少しずつ延ばしながら（1 秒から最大 30 秒）自動的に再試行されます。アプリの起動と同時に接続するようプロファイルを設定することもできます。
- **リアルタイムの状態表示**：サーバーごとの接続状態、マッピングごとの待ち受け状態、開いている接続数、トラフィックのカウンター、そして小さなスループットのグラフ。
- **トレイに常駐**：ウィンドウを閉じてもアプリは終了せず、システムトレイに格納されます（オフにすることもできます）。トレイのメニューからサーバーごとにオン・オフを切り替えられ、サインイン時にアプリを自動起動することもできます。
- **ライトテーマとダークテーマ**。表示言語は 9 つ（英語、簡体字中国語、繁体字中国語、日本語、韓国語、スペイン語、フランス語、ドイツ語、ロシア語）で、選ぶまではシステムの言語に従います。
- **軽量**：Tauri 2 製で、Rust のコアと OS 標準の WebView を使い、ブラウザは同梱しません。

## 仕組み

各マッピングは、手元のマシンのポートを、SSH サーバーから到達できるアドレスへ転送します。

```text
127.0.0.1:8080（手元のマシン）  ──暗号化された SSH──►  127.0.0.1:80（サーバー）
```

これは `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server` と同じです。転送先はサーバー側で解決されるため、`127.0.0.1` はサーバー自身を指します。サーバーから到達できる他のホストも指定できます。

## 既知の制限

- キーボードインタラクティブ（二要素）認証、踏み台サーバー（`ProxyJump`）、ホスト証明書にはまだ対応していません。
- 対応しているのはローカルフォワード（`ssh -L`）のみで、リモートフォワード（`-R`）と SOCKS（`-D`）は使えません。
- `~/.ssh/config` からのインポートは一度きりのコピーで、あとでファイルを編集しても反映されません。`ProxyJump` や `ProxyCommand` が必要なホストは一覧に表示されますが、まだインポートできません。`Match` ブロックはスキップされ、`RemoteForward` と `DynamicForward` は無視されます。`LocalForward` のないホストはフォームが開くので、ポートはそこで選びます。
- GNOME ではトレイアイコンの表示に AppIndicator 拡張機能が必要です。入れていない場合は、設定でトレイ常駐のオプションをオフにするか、アプリをもう一度起動してウィンドウを呼び戻してください。

## 開発

前提条件：

- Node.js（現行の LTS）と npm
- Rust 1.90 以降（[rustup](https://rustup.rs)）
- お使いの OS 向けの Tauri のシステム依存パッケージ。[Tauri の前提条件](https://tauri.app/start/prerequisites/)を参照してください。Debian/Ubuntu の場合：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

続いて：

```bash
npm install
npm run tauri dev    # アプリを起動（ホットリロード対応）
npm run dev          # インターフェースだけをブラウザで実行（SSH の代わりに偽のバックエンドを使用）
npm run tauri build  # インストーラーをビルド（出力先：src-tauri/target/release/bundle/）
npm run typecheck    # フロントエンドの型チェック
cargo test --manifest-path src-tauri/Cargo.toml  # Rust のテストを実行
```

## インストーラーとリリース

パッケージ化は GitHub Actions が行うので、macOS や Windows のマシンを用意する必要はありません。

- **CI**（`.github/workflows/ci.yml`）はすべてのプルリクエストと `main` で実行されます。インターフェースの型チェックとビルドに加え、Linux・macOS・Windows で rustfmt、clippy、Rust のテストを実行します。
- **Release**（`.github/workflows/release.yml`）がインストーラーをビルドします。Linux 向けは `.deb`・`.rpm`・`.AppImage`、macOS 向けは `.dmg`（Apple シリコンと Intel）、Windows 向けは `.exe` と `.msi` です。

バージョンをリリースするには、`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` のバージョン番号を同じ値にしてコミットし、タグを付けてプッシュします。

```bash
git tag v0.2.0
git push origin v0.2.0
```

タグがこの 3 つのバージョンと一致しない場合、ワークフローは中止されます。成功するとインストーラーはドラフトのリリースに添付されるので、GitHub で内容を確認してから公開してください。**Release** を手動で実行する（Actions タブ、Release、Run workflow）と、同じインストーラーがワークフローのアーティファクトとしてダウンロードできるだけで、何も公開されません。

インストーラーにはまだコード署名がありません。macOS 版はアドホック署名なので、初回起動時は右クリックして「開く」を選ぶ（またはシステム設定の「プライバシーとセキュリティ」で許可する）必要があり、Windows では SmartScreen の警告が表示されます（「詳細情報」、「実行」）。本格的な署名と公証には証明書が必要です。証明書は GitHub の Secrets に入れ、リポジトリには決して置かないでください。Tauri の [macOS](https://v2.tauri.app/distribute/sign/macos/) と [Windows](https://v2.tauri.app/distribute/sign/windows/) の署名ガイドを参照してください。

## プロジェクト構成

```text
src/         React + TypeScript のユーザーインターフェース
src-tauri/   Rust コア：SSH ポートフォワーディング、プロファイルの保存、OS の資格情報ストア、トレイ
.github/     CI とリリースのワークフロー
```
