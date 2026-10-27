<div align="center">

# Server Ports

**SSH를 통해 원격 서버의 포트를 내 컴퓨터로 연결합니다.**

[English](../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | 한국어 | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **상태: 초기 개발 중.** 첫 번째 릴리스는 v0.1.0입니다. 터널 엔진과 인터페이스는 갖춰졌고, CI가 Linux, macOS, Windows에서 빌드와 테스트를 거쳐 설치 파일을 만들지만, 지금까지 설치해서 실행해 본 것은 Linux용 `.deb`(Ubuntu에서)뿐입니다. 나머지 설치 파일은 검증하지 않았고, 어느 것도 코드 서명이 되어 있지 않습니다.

Server Ports는 SSH 포트 포워딩을 계속 유지해 주는 작은 크로스플랫폼 데스크톱 앱(Windows, macOS, Linux)입니다. 서버를 고르고 필요한 포트를 나열하면 `localhost`에 바로 나타납니다. `ssh -L`과 같은 일을, 터미널을 열고 명령어를 입력하는 번거로움 없이 할 수 있습니다.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="Server Ports 창: 왼쪽에는 서버 목록, 오른쪽에는 선택한 서버와 그 연결, 그리고 두 개의 포트 포워딩" width="860">
  </picture>
</p>

## 기능

- **서버 프로필**: SSH 서버마다 프로필을 하나씩 만들고, 포트 매핑을 원하는 만큼 추가할 수 있습니다.
- **SSH 설정에서 가져오기**: ‘새 서버’ 대화상자나 시작 화면에서 `~/.ssh/config`의 서버를 골라 한 번에 Server Ports로 복사할 수 있습니다. 호스트 이름, 사용자, 포트, 키 파일(`IdentityFile`), 포트 포워딩(`LocalForward`)이 함께 옮겨지며, `Host *` 블록과 `Include`한 파일에서 상속되는 설정도 반영됩니다. 한 번만 복사하는 방식이며, SSH 설정 파일은 읽기만 하고 변경하지 않습니다.
- **세 가지 로그인 방식**: 비밀번호, 개인 키 파일(암호 구문은 선택), ssh-agent(OpenSSH agent 또는 Pageant).
- **민감한 정보는 설정 파일에 기록하지 않음**: 사용자가 원할 때만 비밀번호와 키 암호 구문이 macOS 키체인, Windows 자격 증명 관리자, Linux Secret Service에 저장되며, 그렇지 않으면 서버에 연결되어 있는 동안에만 메모리에 보관됩니다.
- **호스트 키 검증**: 처음 보는 서버 키는 지문과 함께 표시되고, 사용자가 허락한 경우에만 신뢰합니다. 키가 바뀌면 경고합니다.
- **기본적으로 내 컴퓨터에서만 접근 가능**: 포워딩된 포트는 `127.0.0.1`에서만 수신 대기하므로 내 컴퓨터에서만 접속할 수 있습니다.
- **자동 재연결**: 끊어진 연결은 간격을 점점 늘려 가며(1초에서 최대 30초) 자동으로 다시 시도하며, 앱이 시작될 때 바로 연결되도록 프로필을 설정할 수도 있습니다.
- **실시간 상태**: 서버별 연결 상태, 매핑별 수신 대기 상태, 열려 있는 연결 수, 트래픽 집계, 그리고 작은 처리량 그래프.
- **트레이에 상주**: 창을 닫아도 앱이 종료되지 않고 시스템 트레이로 숨습니다(끌 수 있습니다). 트레이 메뉴에서 서버별로 켜고 끌 수 있으며, 로그인할 때 앱이 자동으로 시작되게 할 수도 있습니다.
- **라이트·다크 테마**, 인터페이스 언어는 영어, 중국어 간체·번체, 일본어, 한국어, 스페인어, 프랑스어, 독일어, 러시아어의 9가지이며, 직접 고르기 전에는 시스템 언어를 따릅니다.
- **가벼움**: Tauri 2 기반으로, Rust 코어와 OS 기본 웹뷰를 사용하며 브라우저를 내장하지 않습니다.

## 설치

[Releases 페이지](https://github.com/gchensy/server-ports/releases)에서 사용하는 시스템에 맞는 설치 파일을 내려받아 실행하세요.

- **Windows**: `.exe` 설치 파일 또는 `.msi`. 설치 파일에는 아직 코드 서명이 없어서 처음 실행할 때 Windows SmartScreen 경고가 나타납니다. ‘추가 정보’를 누른 다음 ‘실행’을 선택하세요.
- **macOS**: `.dmg`(Apple 실리콘은 `aarch64`, Intel은 `x64`). Server Ports를 ‘응용 프로그램’ 폴더로 끌어다 놓으세요. 앱에는 애드혹 서명만 되어 있으므로 처음 실행할 때는 우클릭 후 ‘열기’를 선택하거나, 시스템 설정의 ‘개인정보 보호 및 보안’에서 허용하세요.
- **Linux**: `.deb`(Debian, Ubuntu), `.rpm`(Fedora, openSUSE) 또는 `.AppImage`. 예를 들면 다음과 같습니다.

```bash
sudo apt install ./Server*.deb     # Debian, Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

GNOME에서는 트레이 아이콘에 AppIndicator 확장이 필요합니다(아래 ‘알려진 제한 사항’ 참고). 앱은 스스로 업데이트하지 않습니다. 업그레이드하려면 새 버전을 이전 버전 위에 설치하세요.

## 시작하기

스크린샷은 영어 인터페이스입니다. 본문의 버튼 이름은 한국어 인터페이스의 이름입니다.

### 1. 서버 추가하기

**새 서버**를 클릭하고 양식을 채웁니다.

- **서버**: 호스트(이름 또는 IP 주소), SSH 포트, 사용자. 목록에 표시할 이름도 붙일 수 있습니다.
- **로그인**: **비밀번호**, **키 파일**(**찾아보기…** 버튼으로 개인 키를 선택합니다) 또는 **SSH 에이전트**.
- **포트 포워딩**: **수신 대기**에 내 컴퓨터에서 사용할 포트를 입력하면 서버 쪽 대상 포트(**전달 대상**)가 따라서 바뀌며, 직접 바꿀 수도 있습니다. 필요한 포트마다 포워딩을 추가하고, 원하면 라벨을 붙이세요. **고급**을 누르면 포트가 수신 대기하는 **로컬 주소**가 표시되며, `127.0.0.1`이면 내 컴퓨터에서만 접근할 수 있습니다.
- **옵션**: 연결이 끊어졌을 때 **자동으로 다시 연결**, 그리고 **앱을 시작할 때 연결**.

<p align="center">
  <img src="images/new-server.png" alt="입력을 마친 ‘새 서버’ 대화상자" width="440">
</p>

**저장하고 연결**은 서버를 저장하고 바로 연결하며, **저장**은 저장만 합니다. 나중에는 서버 페이지 오른쪽 위의 연필과 휴지통 버튼으로 편집하거나 삭제할 수 있습니다.

`~/.ssh/config`에 이미 서버가 있다면 대화상자를 **SSH 설정에서**로 전환하고, 가져올 서버에 체크한 뒤 **가져오기**를 클릭하세요. 호스트, 사용자, 포트, 키 파일, `LocalForward` 포트가 함께 들어오며, SSH 설정 파일은 읽기만 할 뿐 절대 바꾸지 않습니다.

<p align="center">
  <img src="images/import-ssh-config.png" alt="SSH 설정에서 찾은 서버가 나열된 ‘새 서버’ 대화상자" width="520">
</p>

### 2. 연결하기

목록에서 서버의 스위치를 켜거나 서버 페이지를 열어 **연결**을 클릭합니다. 처음 연결하는 서버라면 Server Ports가 서버가 제시한 키 종류와 지문을 보여 주고 신뢰할지 묻습니다. 지문을 예상한 값과 대조한 뒤(서버에서 `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub`를 실행하면 출력됩니다) **신뢰하고 계속**을 클릭하세요. 예상한 키가 아니라면 **연결하지 않음**을 선택합니다. 키는 이후에도 기억되며, 나중에 바뀌면 연결이 중단되고 알려 줍니다.

<p align="center">
  <img src="images/trust-server.png" alt="키 종류와 지문이 표시된 ‘이 서버를 신뢰하시겠습니까?’ 질문" width="380">
</p>

서버가 비밀번호를 요구하거나 키에 암호 구문이 걸려 있으면, 이어서 입력을 요청합니다. **시스템 키체인에 기억**을 체크하면 시스템의 자격 증명 저장소에 보관하고, 체크하지 않으면 서버가 연결되어 있는 동안에만 메모리에 둡니다.

### 3. 포트 사용하기

서버가 연결되면 포트 포워딩에 녹색 점이 켜지고 포트가 내 컴퓨터에서 열립니다. 브라우저에서 `http://127.0.0.1:8080`을 열거나, 데이터베이스 클라이언트를 `127.0.0.1:5432`에 연결하는 식으로 어떤 도구든 로컬 포트로 향하게 하세요.

- 포워딩 오른쪽의 두 버튼은 그 주소를 브라우저에서 열거나 복사합니다.
- 주소 아래에는 작은 처리량 그래프, 보내고 받은 데이터량, 현재 열려 있는 연결 수와 누적 연결 수가 표시됩니다.
- **활동** 탭에는 이 서버에서 일어난 일(연결, 재연결, 오류)이 표시됩니다. 뭔가 잘 안 될 때는 먼저 여기를 확인하세요.
- 연결이 끊어지면 Server Ports가 스스로 다시 연결합니다(그 서버에서 자동 재연결을 끄지 않았다면). 기다리는 시간은 매번 조금씩 길어져 최대 30초까지 늘어납니다. **지금 다시 시도**를 누르면 기다리지 않고 바로 시도합니다.

### 4. 계속 실행해 두기

창을 닫아도 터널은 시스템 트레이에서 계속 실행됩니다. 트레이 아이콘을 클릭하거나 트레이 메뉴에서 **Server Ports 열기**를 선택하면 창이 다시 나타나며, 앱을 한 번 더 실행해도 마찬가지입니다. 트레이 메뉴에서는 서버별로 켜고 끄거나 모두 연결하고 모두 연결 끊기를 할 수 있으며, **종료**도 있습니다.

창 왼쪽 아래의 **설정**에서는 언어와 모양을 고르고, 로그인할 때 앱을 시작하게 하거나, 창을 닫을 때 트레이에 남지 않고 앱이 함께 종료되게 할 수 있습니다. **Server Ports 종료**는 앱과 모든 터널을 닫습니다.

<p align="center">
  <img src="images/settings.png" alt="‘설정’ 대화상자" width="360">
</p>

## 동작 방식

각 매핑은 내 컴퓨터의 포트를 SSH 서버가 접근할 수 있는 주소로 전달합니다.

```text
127.0.0.1:8080(내 컴퓨터)  ──암호화된 SSH──►  127.0.0.1:80(서버)
```

이는 `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`와 같습니다. 대상 주소는 서버에서 해석되므로 `127.0.0.1`은 서버 자신을 뜻하며, 서버가 접근할 수 있는 다른 호스트도 지정할 수 있습니다.

## 알려진 제한 사항

- 키보드 대화형(2단계) 인증, 점프 호스트(`ProxyJump`), 호스트 인증서는 아직 지원하지 않습니다.
- 로컬 포워딩(`ssh -L`)만 지원하며, 원격 포워딩(`-R`)과 SOCKS(`-D`)는 지원하지 않습니다.
- `~/.ssh/config` 가져오기는 한 번만 복사하는 방식이라, 이후 파일을 고쳐도 반영되지 않습니다. `ProxyJump`나 `ProxyCommand`가 필요한 호스트는 목록에 표시되지만 아직 가져올 수 없고, `Match` 블록은 건너뛰며, `RemoteForward`와 `DynamicForward`는 무시합니다. `LocalForward`가 없는 호스트는 폼이 열리므로 포트를 직접 고르면 됩니다.
- GNOME에서는 트레이 아이콘을 보려면 AppIndicator 확장이 필요합니다. 없다면 설정에서 트레이 상주 옵션을 끄거나, 앱을 다시 실행해 창을 불러오세요.

## 개발

사전 준비:

- Node.js(현재 LTS)와 npm
- Rust 1.90 이상([rustup](https://rustup.rs))
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
docs/        이 README의 번역본과 거기에 쓰인 스크린샷
.github/     CI 및 릴리스 워크플로
```
