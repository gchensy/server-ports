<div align="center">

# Server Ports

**Asigna puertos de servidores remotos a tu equipo local a través de SSH.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | Español | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Estado: desarrollo inicial.** Todavía no hay ninguna versión publicada. Este README describe lo que la aplicación está diseñada para hacer; una parte aún se está construyendo.

Server Ports es una pequeña aplicación de escritorio multiplataforma (Windows, macOS y Linux) que mantiene activas tus redirecciones de puertos SSH. Elige un servidor, indica los puertos que necesitas y aparecerán en `localhost`. Es lo mismo que `ssh -L`, sin tener que andar con la terminal.

## Características

- **Perfiles de servidor**: un perfil por cada servidor SSH, con tantas asignaciones de puertos como necesites.
- **Tres formas de iniciar sesión**: contraseña, archivo de clave privada (con frase de contraseña opcional) o ssh-agent (agente de OpenSSH o Pageant).
- **Los secretos se quedan en el almacén de credenciales del sistema**: las contraseñas y las frases de contraseña de las claves se guardan en el Llavero de macOS, el Administrador de credenciales de Windows o el Secret Service de Linux, nunca en el archivo de configuración.
- **Verificación de la clave del host**: la clave de un servidor que no habías visto antes se muestra con su huella digital y solo se acepta si tú lo confirmas; si una clave cambia, se avisa.
- **Privado por defecto**: las redirecciones escuchan en `127.0.0.1`, de modo que solo tu propio equipo puede acceder a ellas.
- **Reconexión automática**: las conexiones caídas se reintentan solas, y un perfil puede conectarse en cuanto arranca la aplicación.
- **Estado en tiempo real**: estado de la conexión de cada servidor, estado de escucha de cada asignación, conexiones abiertas y contadores de tráfico.
- **Vive en la bandeja del sistema**: al cerrar la ventana, la aplicación se oculta en la bandeja del sistema en lugar de salir (se puede desactivar).
- **Ligera**: creada con Tauri 2, con un núcleo en Rust y la vista web del sistema, sin navegador incluido.

## Cómo funciona

Cada asignación redirige un puerto de tu equipo a una dirección a la que el servidor SSH puede llegar:

```text
127.0.0.1:8080 (tu equipo)  ──SSH cifrado──►  127.0.0.1:80 (el servidor)
```

Equivale a `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. El destino se resuelve en el servidor, así que `127.0.0.1` es el propio servidor, y también sirve cualquier otro host al que el servidor pueda llegar.

## Desarrollo

Requisitos previos:

- Node.js (la LTS actual) y npm
- Rust 1.89 o posterior ([rustup](https://rustup.rs))
- Las dependencias del sistema de Tauri para tu sistema operativo; consulta los [requisitos previos de Tauri](https://tauri.app/start/prerequisites/). En Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Después:

```bash
npm install
npm run tauri dev    # ejecuta la aplicación con recarga en caliente
npm run tauri build  # genera los instaladores en src-tauri/target/release/bundle/
npm run typecheck    # comprueba los tipos del frontend
cargo test --manifest-path src-tauri/Cargo.toml  # ejecuta las pruebas de Rust
```

## Estructura del proyecto

```text
src/         Interfaz de usuario en React + TypeScript
src-tauri/   Núcleo en Rust: redirección de puertos SSH, almacenamiento de perfiles, almacén de credenciales del sistema, bandeja
```
