<div align="center">

# Server Ports

**通过 SSH 把远程服务器的端口映射到本机。**

[English](../README.md) | 简体中文 | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **状态：早期开发中。** 第一个版本是 v0.1.0。隧道引擎和界面都已就绪，CI 会在 Linux、macOS 和 Windows 上构建、测试并打包，但目前只有 Linux 的 `.deb` 安装并试用过（在 Ubuntu 上）：其他安装包尚未测试，而且都没有代码签名。

Server Ports 是一款小巧的跨平台桌面应用（Windows、macOS、Linux），帮你持续维持 SSH 端口转发。选一台服务器，列出需要的端口，它们就会出现在 `localhost` 上。效果等同于 `ssh -L`，但不用再反复开终端、敲命令。

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="Server Ports 的窗口：左边是服务器列表，右边是选中的服务器、它的连接和两个端口映射" width="860">
  </picture>
</p>

## 功能

- **服务器配置**：每台 SSH 服务器一份配置，可以添加任意多条端口映射。
- **从 SSH 配置导入**：在“新建服务器”对话框或起始页中，从 `~/.ssh/config` 里挑选服务器，一次性复制到 Server Ports。主机名、用户、端口、密钥文件（`IdentityFile`）和端口转发（`LocalForward`）都会一并带入，包括从 `Host *` 块和 `Include` 文件继承来的设置。这只是一次性的复制：SSH 配置文件只会被读取，不会被修改。
- **三种登录方式**：密码、私钥文件（可带口令），或 ssh-agent（OpenSSH agent 或 Pageant）。
- **敏感信息不写进配置文件**：只有在你选择“记住”时，密码和私钥口令才会保存到 macOS 钥匙串、Windows 凭据管理器或 Linux Secret Service；否则它们只在服务器连接期间保存在内存中。
- **主机密钥校验**：第一次见到的服务器密钥会连同指纹一起显示，只有你确认后才会信任；密钥发生变化时会被标记出来。
- **默认只对本机开放**：转发端口监听在 `127.0.0.1` 上，只有本机能访问。
- **自动重连**：连接断开后会自动重试，间隔逐渐拉长（从 1 秒到 30 秒）；也可以让某份配置在应用启动时立即连接。
- **实时状态**：每台服务器的连接状态、每条映射的监听状态、当前连接数、流量统计，以及一张小小的速率曲线。
- **常驻托盘**：关闭窗口时应用会收起到系统托盘而不是退出（可以关闭这一行为），托盘菜单可以逐台开关服务器；也可以设为登录系统时自动启动。
- **浅色和深色主题**，界面提供九种语言：英文、简体中文、繁体中文、日文、韩文、西班牙文、法文、德文和俄文，默认跟随系统语言，也可以手动选择。
- **体积小**：基于 Tauri 2，Rust 内核加系统自带的 WebView，不内置浏览器。

## 安装

从 [Releases 页面](https://github.com/gchensy/server-ports/releases) 下载适合你系统的安装包并运行：

- **Windows**：`.exe` 安装包，或 `.msi`。安装包目前没有代码签名，所以 Windows SmartScreen 会在首次运行时提示警告：选择“更多信息”，再选择“仍要运行”。
- **macOS**：`.dmg`（`aarch64` 适用于 Apple 芯片，`x64` 适用于 Intel）。把 Server Ports 拖进“应用程序”。应用只做了临时（ad hoc）签名，首次打开需要右键选择“打开”，或在“系统设置”的“隐私与安全性”中允许。
- **Linux**：`.deb`（Debian、Ubuntu）、`.rpm`（Fedora、openSUSE）或 `.AppImage`。例如：

```bash
sudo apt install ./Server*.deb     # Debian、Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

在 GNOME 上，托盘图标需要 AppIndicator 扩展（见下面的“已知限制”）。应用不会自动更新：要升级，把新版本直接装在旧版本上即可。

## 快速上手

截图里是英文界面，文中的按钮名称则是简体中文界面里的名称。

### 1. 添加服务器

点击 **新建服务器**，填写表单：

- **服务器**：主机（域名或 IP 地址）、SSH 端口、用户名，也可以起个名字显示在列表里。
- **登录方式**：**密码**、**密钥文件**（用 **浏览…** 选择私钥）或 **SSH 代理**。
- **端口映射**：在 **本机监听** 里填入想在本机使用的端口；服务器上要转发到的端口（**转发到**）会跟着变，你也可以修改。需要几个端口就添加几个映射，还可以给它们加标签。**高级** 会显示端口监听的 **本地地址**：`127.0.0.1` 表示只有本机能访问。
- **选项**：连接断开时 **自动重连**，以及 **应用启动时连接**。

<p align="center">
  <img src="images/new-server.png" alt="填写好的“新建服务器”对话框" width="440">
</p>

**保存并连接** 会保存服务器并立即连接；**保存** 只保存。之后可以用服务器页面右上角的铅笔和垃圾桶按钮来编辑和删除它。

`~/.ssh/config` 里已经有服务器了？把对话框切换到 **从 SSH 配置**，勾选需要的服务器，再点击 **导入**。主机、用户、端口、密钥文件和 `LocalForward` 端口都会一并带过来；你的 SSH 配置文件只会被读取，绝不会被修改。

<p align="center">
  <img src="images/import-ssh-config.png" alt="“新建服务器”对话框，列出了在 SSH 配置中找到的服务器" width="520">
</p>

### 2. 连接

在列表里打开服务器的开关，或进入服务器页面点击 **连接**。第一次连接某台服务器时，Server Ports 会显示它出示的密钥类型和指纹，并询问是否信任。请把指纹与你预期的核对（在服务器上，`ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` 会打印出它），然后点击 **信任并继续**；如果不是你预期的那个，就点击 **不连接**。此后这把密钥会被记住，一旦它发生变化，连接会被中止并提示你。

<p align="center">
  <img src="images/trust-server.png" alt="“信任这台服务器？”的提示，带有密钥类型和指纹" width="380">
</p>

如果服务器用密码登录，或者密钥带有口令，接下来会要求你输入。勾选 **记在系统钥匙串中**，就会把它存进系统的凭据库；否则它只在服务器连接期间保存在内存中。

### 3. 使用端口

服务器连上后，它的端口映射会亮起绿点，端口就在本机打开了：在浏览器里访问 `http://127.0.0.1:8080`，或者让任何工具连接本地端口，比如把数据库客户端连到 `127.0.0.1:5432`。

- 映射右侧的两个按钮，用来在浏览器中打开它的地址，以及复制地址。
- 地址下方是一张小小的吞吐量图、已发送和已接收的数据量，以及当前打开的连接数和累计连接数。
- **活动记录** 页签显示这台服务器上发生的事（连接、重连、错误）：遇到问题先看这里。
- 连接断开时，Server Ports 会自己重连（除非你为这台服务器关闭了自动重连），每次等待的时间会稍长一些，最长 30 秒。点击 **立即重试** 可以跳过等待。

### 4. 让它在后台运行

关闭窗口后，隧道会继续在系统托盘中运行。点击托盘图标，或在它的菜单中选择 **显示 Server Ports**，就能把窗口调回来；再次启动应用也有同样的效果。托盘菜单还能逐台开关服务器、全部连接或全部断开，并有 **退出**。

打开窗口左下角的 **设置**，可以选择语言和外观、设置登录系统时启动应用，或者让应用在关闭窗口时直接退出，而不是留在托盘里。**退出 Server Ports** 会关闭应用和它的所有隧道。

<p align="center">
  <img src="images/settings.png" alt="“设置”对话框" width="360">
</p>

## 工作原理

每条映射都会把本机的一个端口转发到 SSH 服务器能够访问的某个地址：

```text
127.0.0.1:8080（本机）  ──加密的 SSH 连接──►  127.0.0.1:80（服务器）
```

等价于 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`。目标地址由服务器解析，所以 `127.0.0.1` 指的是服务器自己；服务器能访问到的其他主机同样可以作为目标。

## 已知限制

- 暂不支持键盘交互式（双因素）登录、跳板机（`ProxyJump`）和主机证书。
- 只支持本地转发（`ssh -L`），不支持远程转发（`-R`）和 SOCKS 代理（`-D`）。
- 从 `~/.ssh/config` 导入只是一次性复制，之后对该文件的修改不会同步。需要 `ProxyJump` 或 `ProxyCommand` 的主机会列出，但暂时无法导入；`Match` 块会被跳过，`RemoteForward` 和 `DynamicForward` 会被忽略。没有 `LocalForward` 的主机会打开表单，让你自己选择端口。
- 在 GNOME 上，托盘图标需要 AppIndicator 扩展。没有它的话，请在设置里关闭“关闭窗口后继续在托盘中运行”，或者再次启动应用把窗口找回来。

## 开发

前置条件：

- Node.js（当前 LTS 版本）和 npm
- Rust 1.90 或更高版本（[rustup](https://rustup.rs)）
- 当前系统所需的 Tauri 依赖，参见 [Tauri 前置条件](https://tauri.app/start/prerequisites/)。Debian/Ubuntu 上：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

然后：

```bash
npm install
npm run tauri dev    # 运行应用（支持热更新）
npm run dev          # 只在浏览器里运行界面，用模拟后端代替 SSH
npm run tauri build  # 构建安装包，输出到 src-tauri/target/release/bundle/
npm run typecheck    # 前端类型检查
cargo test --manifest-path src-tauri/Cargo.toml  # 运行 Rust 测试
```

## 安装包与发布

打包由 GitHub Actions 完成，不需要自己准备 macOS 或 Windows 的机器。

- **CI**（`.github/workflows/ci.yml`）在每个 Pull Request 和 `main` 上运行：对界面做类型检查和构建，并在 Linux、macOS、Windows 上运行 rustfmt、clippy 和 Rust 测试。
- **Release**（`.github/workflows/release.yml`）负责构建安装包：Linux 的 `.deb`、`.rpm` 和 `.AppImage`，macOS 的 `.dmg`（Apple 芯片和 Intel），Windows 的 `.exe` 和 `.msi`。

发布新版本时，先把 `package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml` 里的版本号改成同一个并提交，然后打标签并推送：

```bash
git tag v0.2.0
git push origin v0.2.0
```

标签与这三处版本号不一致时，工作流会拒绝构建。构建完成后，安装包会附加到一个草稿 Release，在 GitHub 上检查无误再发布。也可以手动运行 **Release**（Actions 页签，Release，Run workflow），它会构建同样的安装包并作为工作流产物供下载，不会发布任何东西。

安装包目前没有代码签名。macOS 版本是临时（ad hoc）签名，首次打开需要右键选择“打开”（或在“系统设置”的“隐私与安全性”中允许），Windows 的 SmartScreen 会提示警告（“更多信息”，“仍要运行”）。正式的签名和公证需要证书，证书应放在 GitHub Secrets 中，绝不能放进仓库，参见 Tauri 的 [macOS](https://v2.tauri.app/distribute/sign/macos/) 和 [Windows](https://v2.tauri.app/distribute/sign/windows/) 签名指南。

## 项目结构

```text
src/         React + TypeScript 用户界面
src-tauri/   Rust 内核：SSH 端口转发、配置存储、系统凭据库、托盘
docs/        本 README 的各语言译文，以及其中用到的截图
.github/     CI 与发布工作流
```
