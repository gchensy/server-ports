<div align="center">

# Server Ports

**通过 SSH 把远程服务器的端口映射到本机。**

[English](../README.md) | 简体中文 | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **状态：早期开发中。** 目前还没有正式发布的版本。隧道引擎和界面都已就绪，CI 会在 Linux、macOS 和 Windows 上构建、测试并打包，但应用目前只在 Linux 上运行过，macOS 和 Windows 的安装包尚未测试。

Server Ports 是一款小巧的跨平台桌面应用（Windows、macOS、Linux），帮你持续维持 SSH 端口转发。选一台服务器，列出需要的端口，它们就会出现在 `localhost` 上。效果等同于 `ssh -L`，但不用再反复开终端、敲命令。

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
.github/     CI 与发布工作流
```
