<div align="center">

# Server Ports

**SSH를 통해 원격 서버의 포트를 내 컴퓨터로 연결합니다.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | 한국어 | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **상태: 초기 개발 중.** 아직 정식 릴리스는 없습니다. 터널 엔진과 인터페이스는 갖춰졌고, CI가 세 운영체제용 설치 파일을 빌드하지만, 지금까지 Linux에서만 실행해 보았으며 macOS와 Windows는 검증하지 않았습니다.

Server Ports는 SSH 포트 포워딩을 계속 유지해 주는 작은 크로스플랫폼 데스크톱 앱(Windows, macOS, Linux)입니다. 서버를 고르고 필요한 포트를 나열하면 `localhost`에 바로 나타납니다. `ssh -L`과 같은 일을, 터미널을 열고 명령어를 입력하는 번거로움 없이 할 수 있습니다.

## 기능

- **서버 프로필**: SSH 서버마다 프로필을 하나씩 만들고, 포트 매핑을 원하는 만큼 추가할 수 있습니다.
- **세 가지 로그인 방식**: 비밀번호, 개인 키 파일(암호 구문은 선택), ssh-agent(OpenSSH agent 또는 Pageant).
- **민감한 정보는 설정 파일에 기록하지 않음**: 사용자가 원할 때만 비밀번호와 키 암호 구문이 macOS 키체인, Windows 자격 증명 관리자, Linux Secret Service에 저장되며, 그렇지 않으면 서버에 연결되어 있는 동안에만 메모리에 보관됩니다.
- **호스트 키 검증**: 처음 보는 서버 키는 지문과 함께 표시되고, 사용자가 허락한 경우에만 신뢰합니다. 키가 바뀌면 경고합니다.
- **기본적으로 내 컴퓨터에서만 접근 가능**: 포워딩된 포트는 `127.0.0.1`에서만 수신 대기하므로 내 컴퓨터에서만 접속할 수 있습니다.
- **자동 재연결**: 끊어진 연결은 간격을 점점 늘려 가며(1초에서 최대 30초) 자동으로 다시 시도하며, 앱이 시작될 때 바로 연결되도록 프로필을 설정할 수도 있습니다.
- **실시간 상태**: 서버별 연결 상태, 매핑별 수신 대기 상태, 열려 있는 연결 수, 트래픽 집계, 그리고 작은 처리량 그래프.
- **트레이에 상주**: 창을 닫아도 앱이 종료되지 않고 시스템 트레이로 숨습니다(끌 수 있습니다). 트레이 메뉴에서 서버별로 켜고 끌 수 있으며, 로그인할 때 앱이 자동으로 시작되게 할 수도 있습니다.
- **라이트·다크 테마**, 인터페이스 언어는 영어, 중국어 간체·번체, 일본어, 한국어, 스페인어, 프랑스어, 독일어, 러시아어의 9가지이며, 직접 고르기 전에는 시스템 언어를 따릅니다.
- **가벼움**: Tauri 2 기반으로, Rust 코어와 OS 기본 웹뷰를 사용하며 브라우저를 내장하지 않습니다.

## 동작 방식

각 매핑은 내 컴퓨터의 포트를 SSH 서버가 접근할 수 있는 주소로 전달합니다.

```text
127.0.0.1:8080(내 컴퓨터)  ──암호화된 SSH──►  127.0.0.1:80(서버)
```

이는 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`와 같습니다. 대상 주소는 서버에서 해석되므로 `127.0.0.1`은 서버 자신을 뜻하며, 서버가 접근할 수 있는 다른 호스트도 지정할 수 있습니다.

## 알려진 제한 사항

- 키보드 대화형(2단계) 인증, 점프 호스트(`ProxyJump`), 호스트 인증서는 아직 지원하지 않습니다.
- 로컬 포워딩(`ssh -L`)만 지원하며, 원격 포워딩(`-R`)과 SOCKS(`-D`)는 지원하지 않습니다.
- 서버는 직접 추가해야 하며 `~/.ssh/config`는 읽지 않습니다.
- GNOME에서는 트레이 아이콘을 보려면 AppIndicator 확장이 필요합니다. 없다면 설정에서 트레이 상주 옵션을 끄거나, 앱을 다시 실행해 창을 불러오세요.

## 개발

사전 준비:

- Node.js(현재 LTS)와 npm
- Rust 1.89 이상([rustup](https://rustup.rs))
- 사용 중인 OS에 필요한 Tauri 시스템 의존성. [Tauri 사전 요구 사항](https://tauri.app/start/prerequisites/)을 참고하세요. Debian/Ubuntu의 경우:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

그다음:

```bash
npm install
npm run tauri dev    # 앱 실행(핫 리로드 지원)
npm run dev          # 인터페이스만 브라우저에서 실행(SSH 대신 가짜 백엔드 사용)
npm run tauri build  # 설치 파일 빌드(출력 위치: src-tauri/target/release/bundle/)
npm run typecheck    # 프런트엔드 타입 검사
cargo test --manifest-path src-tauri/Cargo.toml  # Rust 테스트 실행
```

## 설치 파일과 릴리스

패키징은 GitHub Actions가 맡으므로 macOS나 Windows 컴퓨터를 따로 준비할 필요가 없습니다.

- **CI**(`.github/workflows/ci.yml`)는 모든 풀 리퀘스트와 `main`에서 실행됩니다. 인터페이스의 타입 검사와 빌드, 그리고 Linux·macOS·Windows에서 rustfmt, clippy, Rust 테스트를 수행합니다.
- **Release**(`.github/workflows/release.yml`)가 설치 파일을 빌드합니다. Linux용은 `.deb`, `.rpm`, `.AppImage`, macOS용은 `.dmg`(Apple 실리콘과 Intel), Windows용은 `.exe`와 `.msi`입니다.

새 버전을 릴리스하려면 `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`의 버전을 같은 값으로 맞춰 커밋한 뒤 태그를 만들어 푸시합니다.

```bash
git tag v0.2.0
git push origin v0.2.0
```

태그가 이 세 파일의 버전과 다르면 워크플로가 중단됩니다. 성공하면 설치 파일이 초안 릴리스에 첨부되며, GitHub에서 확인한 뒤 공개하면 됩니다. **Release**를 직접 실행하면(Actions 탭, Release, Run workflow) 같은 설치 파일이 워크플로 아티팩트로 내려받을 수 있게 만들어질 뿐, 아무것도 릴리스되지 않습니다.

설치 파일에는 아직 코드 서명이 없습니다. macOS 빌드는 애드혹 서명이라 처음 실행할 때 우클릭 후 ‘열기’를 선택하거나(또는 시스템 설정의 ‘개인정보 보호 및 보안’에서 허용) 해야 하고, Windows에서는 SmartScreen 경고가 나타납니다(‘추가 정보’, ‘실행’). 정식 서명과 공증에는 인증서가 필요하며, 인증서는 GitHub Secrets에 두고 저장소에는 절대 넣지 마세요. Tauri의 [macOS](https://v2.tauri.app/distribute/sign/macos/), [Windows](https://v2.tauri.app/distribute/sign/windows/) 서명 가이드를 참고하세요.

## 프로젝트 구조

```text
src/         React + TypeScript 사용자 인터페이스
src-tauri/   Rust 코어: SSH 포트 포워딩, 프로필 저장, OS 자격 증명 저장소, 트레이
.github/     CI 및 릴리스 워크플로
```
