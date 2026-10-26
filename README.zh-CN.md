<div align="center">

# Server Ports

**通过 SSH 把远程服务器的端口映射到本机。**

[English](README.md) | 简体中文 | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **状态：早期开发中。** 目前还没有正式发布的版本。隧道引擎和界面都已就绪，但应用目前只在 Linux 上构建和运行过，macOS 和 Windows 尚未测试。

Server Ports 是一款小巧的跨平台桌面应用（Windows、macOS、Linux），帮你持续维持 SSH 端口转发。选一台服务器，列出需要的端口，它们就会出现在 `localhost` 上。效果等同于 `ssh -L`，但不用再反复开终端、敲命令。

## 功能

- **服务器配置**：每台 SSH 服务器一份配置，可以添加任意多条端口映射。
- **三种登录方式**：密码、私钥文件（可带口令），或 ssh-agent（OpenSSH agent 或 Pageant）。
- **敏感信息不写进配置文件**：只有在你选择“记住”时，密码和私钥口令才会保存到 macOS 钥匙串、Windows 凭据管理器或 Linux Secret Service；否则它们只在服务器连接期间保存在内存中。
- **主机密钥校验**：第一次见到的服务器密钥会连同指纹一起显示，只有你确认后才会信任；密钥发生变化时会被标记出来。
- **默认只对本机开放**：转发端口监听在 `127.0.0.1` 上，只有本机能访问。
- **自动重连**：连接断开后会自动重试，间隔逐渐拉长（从 1 秒到 30 秒）；也可以让某份配置在应用启动时立即连接。
- **实时状态**：每台服务器的连接状态、每条映射的监听状态、当前连接数、流量统计，以及一张小小的速率曲线。
- **常驻托盘**：关闭窗口时应用会收起到系统托盘而不是退出（可以关闭这一行为），托盘菜单可以逐台开关服务器；也可以设为登录系统时自动启动。
- **浅色和深色主题**，界面提供英文和简体中文。
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
- 服务器需要手动添加，不会读取 `~/.ssh/config`。
- 在 GNOME 上，托盘图标需要 AppIndicator 扩展。没有它的话，请在设置里关闭“关闭窗口后继续在托盘中运行”，或者再次启动应用把窗口找回来。

## 开发

前置条件：

- Node.js（当前 LTS 版本）和 npm
- Rust 1.89 或更高版本（[rustup](https://rustup.rs)）
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

## 项目结构

```text
src/         React + TypeScript 用户界面
src-tauri/   Rust 内核：SSH 端口转发、配置存储、系统凭据库、托盘
```
