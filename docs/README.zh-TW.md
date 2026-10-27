<div align="center">

# Server Ports

**透過 SSH 將遠端伺服器的連接埠對應到本機。**

[English](../README.md) | [简体中文](README.zh-CN.md) | 繁體中文 | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **狀態：早期開發中。** 第一個版本是 v0.1.0。通道引擎與介面都已就緒，CI 會在 Linux、macOS 與 Windows 上建置、測試並打包，但目前只有 Linux 的 `.deb` 安裝並試用過（在 Ubuntu 上）：其他安裝程式尚未測試，而且都沒有程式碼簽章。

Server Ports 是一款小巧的跨平台桌面應用程式（Windows、macOS、Linux），幫你持續維持 SSH 連接埠轉送。選一台伺服器，列出需要的連接埠，它們就會出現在 `localhost` 上。效果等同於 `ssh -L`，但不必再反覆開啟終端機、輸入指令。

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="Server Ports 的視窗：左邊是伺服器清單，右邊是選取的伺服器、它的連線與兩個連接埠對應" width="860">
  </picture>
</p>

## 功能

- **伺服器設定檔**：每台 SSH 伺服器一份設定檔，可新增任意數量的連接埠對應。
- **從 SSH 設定匯入**：在「新增伺服器」對話框或起始畫面中，從 `~/.ssh/config` 挑選伺服器，一次複製到 Server Ports。主機名稱、使用者、連接埠、金鑰檔案（`IdentityFile`）與連接埠轉送（`LocalForward`）都會一併帶入，包括從 `Host *` 區塊與 `Include` 檔案繼承的設定。這只是一次性的複製：SSH 設定檔只會被讀取，不會被修改。
- **三種登入方式**：密碼、私密金鑰檔案（可搭配通關密語），或 ssh-agent（OpenSSH agent 或 Pageant）。
- **機密資料不寫入設定檔**：只有在你選擇「記住」時，密碼與金鑰通關密語才會存放到 macOS 鑰匙圈、Windows 認證管理員或 Linux Secret Service；否則它們只會在伺服器連線期間保留在記憶體中。
- **主機金鑰驗證**：第一次看到的伺服器金鑰會連同指紋一併顯示，唯有你確認後才會信任；金鑰若有變動，也會被標示出來。
- **預設僅限本機存取**：轉送的連接埠只監聽 `127.0.0.1`，只有本機能連上。
- **自動重新連線**：連線中斷後會自動重試，間隔逐漸拉長（從 1 秒到 30 秒）；也可以讓某份設定檔在應用程式啟動時立即連線。
- **即時狀態**：每台伺服器的連線狀態、每個對應的監聽狀態、目前連線數、流量統計，以及一張小小的速率圖。
- **常駐系統匣**：關閉視窗時，應用程式會縮到系統匣而不是結束（可關閉此行為），系統匣選單可逐台開關伺服器；也可以設定為登入系統時自動啟動。
- **淺色與深色主題**，介面提供九種語言：英文、簡體中文、繁體中文、日文、韓文、西班牙文、法文、德文與俄文，預設跟隨系統語言，也可以手動選擇。
- **體積小**：以 Tauri 2 打造，Rust 核心搭配系統內建的 WebView，不內附瀏覽器。

## 安裝

從 [Releases 頁面](https://github.com/gchensy/server-ports/releases) 下載適合你系統的安裝程式並執行：

- **Windows**：`.exe` 安裝程式，或 `.msi`。安裝程式目前沒有程式碼簽章，所以 Windows SmartScreen 會在第一次執行時顯示警告：選擇「其他資訊」，再選擇「仍要執行」。
- **macOS**：`.dmg`（`aarch64` 適用於 Apple 晶片，`x64` 適用於 Intel）。把 Server Ports 拖進「應用程式」。應用程式只做了臨時（ad hoc）簽章，第一次開啟需要按右鍵選擇「打開」，或在「系統設定」的「隱私權與安全性」中允許。
- **Linux**：`.deb`（Debian、Ubuntu）、`.rpm`（Fedora、openSUSE）或 `.AppImage`。例如：

```bash
sudo apt install ./Server*.deb     # Debian、Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

在 GNOME 上，系統匣圖示需要 AppIndicator 擴充功能（見下方的「已知限制」）。應用程式不會自動更新：要升級，把新版本直接安裝在舊版本上即可。

## 快速上手

螢幕截圖是英文介面，文中的按鈕名稱則是繁體中文介面上的名稱。

### 1. 新增伺服器

點選 **新增伺服器**，填寫表單：

- **伺服器**：主機（網域名稱或 IP 位址）、SSH 連接埠、使用者，也可以取個名字顯示在清單裡。
- **登入方式**：**密碼**、**金鑰檔案**（用 **瀏覽…** 選擇私密金鑰）或 **SSH 代理程式**。
- **連接埠對應**：在 **本機監聽** 填入想在本機使用的連接埠；伺服器上要轉送到的連接埠（**轉送到**）會跟著變，你也可以修改。需要幾個連接埠就新增幾個對應，還可以加上標籤。**進階** 會顯示連接埠監聽的 **本機位址**：`127.0.0.1` 表示只有本機能存取。
- **選項**：連線中斷時 **自動重新連線**，以及 **應用程式啟動時連線**。

<p align="center">
  <img src="images/new-server.png" alt="填寫好的「新增伺服器」對話方塊" width="440">
</p>

**儲存並連線** 會儲存伺服器並立刻連線；**儲存** 只儲存。之後可以用伺服器頁面右上角的鉛筆與垃圾桶按鈕來編輯與刪除它。

`~/.ssh/config` 裡已經有伺服器了嗎？把對話方塊切換到 **從 SSH 設定**，勾選需要的伺服器，再點選 **匯入**。主機、使用者、連接埠、金鑰檔案與 `LocalForward` 連接埠都會一併帶過來；你的 SSH 設定檔只會被讀取，絕不會被修改。

<p align="center">
  <img src="images/import-ssh-config.png" alt="「新增伺服器」對話方塊，列出在 SSH 設定中找到的伺服器" width="520">
</p>

### 2. 連線

在清單裡開啟伺服器的開關，或進入伺服器頁面點選 **連線**。第一次連線到某台伺服器時，Server Ports 會顯示它出示的金鑰類型與指紋，並詢問是否信任。請把指紋與你預期的核對（在伺服器上，`ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` 會印出它），然後點選 **信任並繼續**；如果不是你預期的那一個，就點選 **不要連線**。此後這把金鑰會被記住，一旦它有變動，連線會被中止並通知你。

<p align="center">
  <img src="images/trust-server.png" alt="「要信任這台伺服器嗎？」的提示，附有金鑰類型與指紋" width="380">
</p>

如果伺服器用密碼登入，或金鑰設有通關密語，接下來會請你輸入。勾選 **記在系統鑰匙圈中**，就會把它存進系統的認證儲存區；否則它只在伺服器連線期間保留在記憶體中。

### 3. 使用連接埠

伺服器連上後，它的連接埠對應會亮起綠點，連接埠就在本機開啟了：在瀏覽器裡開啟 `http://127.0.0.1:8080`，或讓任何工具連到本機連接埠，例如把資料庫用戶端連到 `127.0.0.1:5432`。

- 對應右側的兩個按鈕，用來在瀏覽器中開啟它的位址，以及複製位址。
- 位址下方是一張小小的速率圖、已傳送與已接收的資料量，以及目前開啟的連線數與累計連線數。
- **活動紀錄** 頁籤會顯示這台伺服器上發生的事（連線、重新連線、錯誤）：遇到問題先看這裡。
- 連線中斷時，Server Ports 會自己重新連線（除非你為這台伺服器關閉了自動重新連線），每次等待的時間會稍微長一些，最長 30 秒。點選 **立即重試** 可以跳過等待。

### 4. 讓它在背景執行

關閉視窗後，通道會繼續在系統匣中執行。點選系統匣圖示，或在它的選單中選擇 **顯示 Server Ports**，就能把視窗叫回來；再次啟動應用程式也有同樣的效果。系統匣選單還能逐台開關伺服器、全部連線或全部中斷連線，並有 **結束**。

開啟視窗左下角的 **設定**，可以選擇語言與外觀、設定登入時啟動應用程式，或讓應用程式在關閉視窗時直接結束，而不是留在系統匣裡。**結束 Server Ports** 會關閉應用程式與它的所有通道。

<p align="center">
  <img src="images/settings.png" alt="「設定」對話方塊" width="360">
</p>

## 運作原理

每個對應都會把本機的一個連接埠轉送到 SSH 伺服器能夠連到的某個位址：

```text
127.0.0.1:8080（本機）  ──加密的 SSH 連線──►  127.0.0.1:80（伺服器）
```

等同於 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`。目標位址由伺服器解析，所以 `127.0.0.1` 指的是伺服器本身；伺服器能連到的其他主機同樣可以作為目標。

## 已知限制

- 尚不支援鍵盤互動式（雙因素）登入、跳板機（`ProxyJump`）與主機憑證。
- 僅支援本機轉送（`ssh -L`），不支援遠端轉送（`-R`）與 SOCKS 代理（`-D`）。
- 從 `~/.ssh/config` 匯入只是一次性的複製，之後對該檔案的修改不會同步。需要 `ProxyJump` 或 `ProxyCommand` 的主機會列出，但目前無法匯入；`Match` 區塊會被略過，`RemoteForward` 與 `DynamicForward` 會被忽略。沒有 `LocalForward` 的主機會開啟表單，讓你自行選擇連接埠。
- 在 GNOME 上，系統匣圖示需要 AppIndicator 擴充功能。沒有它的話，請在設定中關閉系統匣常駐的選項，或再次啟動應用程式把視窗找回來。

## 開發

前置需求：

- Node.js（目前的 LTS 版本）與 npm
- Rust 1.90 或更新版本（[rustup](https://rustup.rs)）
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
docs/        本 README 的各語言譯文，以及其中用到的螢幕截圖
.github/     CI 與發行工作流程
```
