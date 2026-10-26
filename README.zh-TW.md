<div align="center">

# Server Ports

**透過 SSH 將遠端伺服器的連接埠對應到本機。**

[English](README.md) | [简体中文](README.zh-CN.md) | 繁體中文 | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **狀態：早期開發中。** 目前尚無正式發行的版本。通道引擎與介面都已就緒，CI 會在 Linux、macOS 與 Windows 上建置、測試並打包，但應用程式目前只在 Linux 上執行過，macOS 與 Windows 的安裝程式尚未測試。

Server Ports 是一款小巧的跨平台桌面應用程式（Windows、macOS、Linux），幫你持續維持 SSH 連接埠轉送。選一台伺服器，列出需要的連接埠，它們就會出現在 `localhost` 上。效果等同於 `ssh -L`，但不必再反覆開啟終端機、輸入指令。

## 功能

- **伺服器設定檔**：每台 SSH 伺服器一份設定檔，可新增任意數量的連接埠對應。
- **三種登入方式**：密碼、私密金鑰檔案（可搭配通關密語），或 ssh-agent（OpenSSH agent 或 Pageant）。
- **機密資料不寫入設定檔**：只有在你選擇「記住」時，密碼與金鑰通關密語才會存放到 macOS 鑰匙圈、Windows 認證管理員或 Linux Secret Service；否則它們只會在伺服器連線期間保留在記憶體中。
- **主機金鑰驗證**：第一次看到的伺服器金鑰會連同指紋一併顯示，唯有你確認後才會信任；金鑰若有變動，也會被標示出來。
- **預設僅限本機存取**：轉送的連接埠只監聽 `127.0.0.1`，只有本機能連上。
- **自動重新連線**：連線中斷後會自動重試，間隔逐漸拉長（從 1 秒到 30 秒）；也可以讓某份設定檔在應用程式啟動時立即連線。
- **即時狀態**：每台伺服器的連線狀態、每個對應的監聽狀態、目前連線數、流量統計，以及一張小小的速率圖。
- **常駐系統匣**：關閉視窗時，應用程式會縮到系統匣而不是結束（可關閉此行為），系統匣選單可逐台開關伺服器；也可以設定為登入系統時自動啟動。
- **淺色與深色主題**，介面提供九種語言：英文、簡體中文、繁體中文、日文、韓文、西班牙文、法文、德文與俄文，預設跟隨系統語言，也可以手動選擇。
- **體積小**：以 Tauri 2 打造，Rust 核心搭配系統內建的 WebView，不內附瀏覽器。

## 運作原理

每個對應都會把本機的一個連接埠轉送到 SSH 伺服器能夠連到的某個位址：

```text
127.0.0.1:8080（本機）  ──加密的 SSH 連線──►  127.0.0.1:80（伺服器）
```

等同於 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`。目標位址由伺服器解析，所以 `127.0.0.1` 指的是伺服器本身；伺服器能連到的其他主機同樣可以作為目標。

## 已知限制

- 尚不支援鍵盤互動式（雙因素）登入、跳板機（`ProxyJump`）與主機憑證。
- 僅支援本機轉送（`ssh -L`），不支援遠端轉送（`-R`）與 SOCKS 代理（`-D`）。
- 伺服器需要手動新增，不會讀取 `~/.ssh/config`。
- 在 GNOME 上，系統匣圖示需要 AppIndicator 擴充功能。沒有它的話，請在設定中關閉系統匣常駐的選項，或再次啟動應用程式把視窗找回來。

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
npm run dev          # 只在瀏覽器中執行介面，以模擬後端取代 SSH
npm run tauri build  # 建置安裝程式，輸出到 src-tauri/target/release/bundle/
npm run typecheck    # 前端型別檢查
cargo test --manifest-path src-tauri/Cargo.toml  # 執行 Rust 測試
```

## 安裝程式與發行

打包由 GitHub Actions 負責，不需要自己準備 macOS 或 Windows 的機器。

- **CI**（`.github/workflows/ci.yml`）會在每個 Pull Request 與 `main` 上執行：對介面做型別檢查與建置，並在 Linux、macOS、Windows 上執行 rustfmt、clippy 與 Rust 測試。
- **Release**（`.github/workflows/release.yml`）負責建置安裝程式：Linux 的 `.deb`、`.rpm` 與 `.AppImage`，macOS 的 `.dmg`（Apple 晶片與 Intel），Windows 的 `.exe` 與 `.msi`。

發行新版本時，先把 `package.json`、`src-tauri/tauri.conf.json` 與 `src-tauri/Cargo.toml` 裡的版本號改成同一個並提交，然後建立標籤並推送：

```bash
git tag v0.2.0
git push origin v0.2.0
```

標籤與這三處版本號不一致時，工作流程會拒絕建置。建置完成後，安裝程式會附加到一個草稿 Release，在 GitHub 上確認無誤再發布。也可以手動執行 **Release**（Actions 分頁，Release，Run workflow），它會建置相同的安裝程式並作為工作流程產出物供下載，不會發布任何東西。

安裝程式目前沒有程式碼簽章。macOS 版本是臨時（ad hoc）簽章，第一次開啟需要按右鍵選擇「打開」（或在「系統設定」的「隱私權與安全性」中允許），Windows 的 SmartScreen 會顯示警告（「其他資訊」，「仍要執行」）。正式的簽章與公證需要憑證，憑證應放在 GitHub Secrets 中，絕不能放進儲存庫，請參考 Tauri 的 [macOS](https://v2.tauri.app/distribute/sign/macos/) 與 [Windows](https://v2.tauri.app/distribute/sign/windows/) 簽章指南。

## 專案結構

```text
src/         React + TypeScript 使用者介面
src-tauri/   Rust 核心：SSH 連接埠轉送、設定檔儲存、系統認證儲存區、系統匣
.github/     CI 與發行工作流程
```
