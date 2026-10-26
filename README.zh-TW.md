<div align="center">

# Server Ports

**透過 SSH 將遠端伺服器的連接埠對應到本機。**

[English](README.md) | [简体中文](README.zh-CN.md) | 繁體中文 | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **狀態：早期開發中。** 目前尚無正式發行的版本。本文說明的是應用程式的設計目標，其中部分功能仍在開發。

Server Ports 是一款小巧的跨平台桌面應用程式（Windows、macOS、Linux），幫你持續維持 SSH 連接埠轉送。選一台伺服器，列出需要的連接埠，它們就會出現在 `localhost` 上。效果等同於 `ssh -L`，但不必再反覆開啟終端機、輸入指令。

## 功能

- **伺服器設定檔**：每台 SSH 伺服器一份設定檔，可新增任意數量的連接埠對應。
- **三種登入方式**：密碼、私密金鑰檔案（可搭配通關密語），或 ssh-agent（OpenSSH agent 或 Pageant）。
- **機密資料只存放在系統認證儲存區**：密碼與金鑰通關密語會存放在 macOS 鑰匙圈、Windows 認證管理員或 Linux Secret Service，不會寫入設定檔。
- **主機金鑰驗證**：第一次看到的伺服器金鑰會連同指紋一併顯示，唯有你確認後才會信任；金鑰若有變動，也會被標示出來。
- **預設僅限本機存取**：轉送的連接埠只監聽 `127.0.0.1`，只有本機能連上。
- **自動重新連線**：連線中斷後會自動重試；也可以讓某份設定檔在應用程式啟動時立即連線。
- **即時狀態**：每台伺服器的連線狀態、每個對應的監聽狀態、目前連線數與流量統計。
- **常駐系統匣**：關閉視窗時，應用程式會縮到系統匣而不是結束（可關閉此行為）。
- **體積小**：以 Tauri 2 打造，Rust 核心搭配系統內建的 WebView，不內附瀏覽器。

## 運作原理

每個對應都會把本機的一個連接埠轉送到 SSH 伺服器能夠連到的某個位址：

```text
127.0.0.1:8080（本機）  ──加密的 SSH 連線──►  127.0.0.1:80（伺服器）
```

等同於 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`。目標位址由伺服器解析，所以 `127.0.0.1` 指的是伺服器本身；伺服器能連到的其他主機同樣可以作為目標。

## 開發

前置需求：

- Node.js（目前的 LTS 版本）與 npm
- Rust 1.89 或更新版本（[rustup](https://rustup.rs)）
- 你的作業系統所需的 Tauri 相依套件，請參閱 [Tauri 前置需求](https://tauri.app/start/prerequisites/)。在 Debian/Ubuntu 上：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

接著：

```bash
npm install
npm run tauri dev    # 執行應用程式（支援熱重載）
npm run tauri build  # 建置安裝程式，輸出到 src-tauri/target/release/bundle/
npm run typecheck    # 前端型別檢查
cargo test --manifest-path src-tauri/Cargo.toml  # 執行 Rust 測試
```

## 專案結構

```text
src/         React + TypeScript 使用者介面
src-tauri/   Rust 核心：SSH 連接埠轉送、設定檔儲存、系統認證儲存區、系統匣
```
