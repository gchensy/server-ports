<div align="center">

# Server Ports

**Asigna puertos de servidores remotos a tu equipo local a través de SSH.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | Español | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Estado: desarrollo inicial.** Todavía no hay ninguna versión publicada. El motor de túneles y la interfaz ya existen, y la integración continua (CI) compila instaladores para los tres sistemas, pero la aplicación solo se ha ejecutado en Linux; macOS y Windows no se han probado.

Server Ports es una pequeña aplicación de escritorio multiplataforma (Windows, macOS y Linux) que mantiene activas tus redirecciones de puertos SSH. Elige un servidor, indica los puertos que necesitas y aparecerán en `localhost`. Es lo mismo que `ssh -L`, sin tener que andar con la terminal.

## Características

- **Perfiles de servidor**: un perfil por cada servidor SSH, con tantas asignaciones de puertos como necesites.
- **Tres formas de iniciar sesión**: contraseña, archivo de clave privada (con frase de contraseña opcional) o ssh-agent (agente de OpenSSH o Pageant).
- **Los secretos no van al archivo de configuración**: si lo pides, las contraseñas y las frases de contraseña de las claves se guardan en el Llavero de macOS, el Administrador de credenciales de Windows o el Secret Service de Linux; si no, solo se conservan en memoria mientras el servidor está conectado.
- **Verificación de la clave del host**: la clave de un servidor que no habías visto antes se muestra con su huella digital y solo se acepta si tú lo confirmas; si una clave cambia, se avisa.
- **Privado por defecto**: las redirecciones escuchan en `127.0.0.1`, de modo que solo tu propio equipo puede acceder a ellas.
- **Reconexión automática**: las conexiones caídas se reintentan solas, con una espera cada vez mayor (de 1 s hasta 30 s), y un perfil puede conectarse en cuanto arranca la aplicación.
- **Estado en tiempo real**: estado de la conexión de cada servidor, estado de escucha de cada asignación, conexiones abiertas, contadores de tráfico y un pequeño gráfico de rendimiento.
- **Vive en la bandeja del sistema**: al cerrar la ventana, la aplicación se oculta en la bandeja del sistema en lugar de salir (se puede desactivar), y el menú de la bandeja activa o desactiva cada servidor. La aplicación también puede iniciarse al iniciar sesión.
- **Tema claro y oscuro**, con la interfaz en nueve idiomas: inglés, chino simplificado y tradicional, japonés, coreano, español, francés, alemán y ruso. Sigue el idioma del sistema hasta que elijas uno.
- **Ligera**: creada con Tauri 2, con un núcleo en Rust y la vista web del sistema, sin navegador incluido.

## Cómo funciona

Cada asignación redirige un puerto de tu equipo a una dirección a la que el servidor SSH puede llegar:

```text
127.0.0.1:8080 (tu equipo)  ──SSH cifrado──►  127.0.0.1:80 (el servidor)
```

Equivale a `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. El destino se resuelve en el servidor, así que `127.0.0.1` es el propio servidor, y también sirve cualquier otro host al que el servidor pueda llegar.

## Limitaciones conocidas

- Todavía no admite el inicio de sesión interactivo con teclado (doble factor), los saltos a través de otros servidores (`ProxyJump`) ni los certificados de host.
- Solo redirecciones locales (`ssh -L`); las remotas (`-R`) y las SOCKS (`-D`) no están soportadas.
- Los servidores se añaden a mano; `~/.ssh/config` no se lee.
- En GNOME, el icono de la bandeja necesita la extensión AppIndicator. Sin ella, desactiva la opción de la bandeja en los ajustes o vuelve a iniciar la aplicación para recuperar su ventana.

## Desarrollo

Requisitos previos:

- Node.js (la LTS actual) y npm
- Rust 1.90 o posterior ([rustup](https://rustup.rs))
- Las dependencias del sistema de Tauri para tu sistema operativo; consulta los [requisitos previos de Tauri](https://tauri.app/start/prerequisites/). En Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Después:

```bash
npm install
npm run tauri dev    # ejecuta la aplicación con recarga en caliente
npm run dev          # solo la interfaz en un navegador, con un backend simulado en lugar de SSH
npm run tauri build  # genera los instaladores en src-tauri/target/release/bundle/
npm run typecheck    # comprueba los tipos del frontend
cargo test --manifest-path src-tauri/Cargo.toml  # ejecuta las pruebas de Rust
```

## Instaladores y versiones

Del empaquetado se encarga GitHub Actions, así que no necesitas ningún equipo con macOS o Windows.

- **CI** (`.github/workflows/ci.yml`) se ejecuta en cada pull request y en `main`: comprueba los tipos y compila la interfaz, y ejecuta rustfmt, clippy y las pruebas de Rust en Linux, macOS y Windows.
- **Release** (`.github/workflows/release.yml`) compila los instaladores: `.deb`, `.rpm` y `.AppImage` para Linux, `.dmg` para macOS (Apple silicon e Intel), `.exe` y `.msi` para Windows.

Para publicar una versión, pon el mismo número de versión en `package.json`, `src-tauri/tauri.conf.json` y `src-tauri/Cargo.toml`, haz commit, crea la etiqueta y súbela:

```bash
git tag v0.2.0
git push origin v0.2.0
```

El flujo de trabajo rechaza una etiqueta que no coincida con esas tres versiones. Después adjunta los instaladores a un borrador de versión, que revisas y publicas en GitHub. Si ejecutas **Release** a mano (pestaña Actions, Release, Run workflow), compila los mismos instaladores como artefactos descargables del flujo de trabajo, sin publicar nada.

Los instaladores todavía no están firmados. Las compilaciones de macOS llevan una firma ad hoc, así que la primera vez hay que hacer clic derecho y elegir Abrir (o permitirlo en Ajustes del Sistema, Privacidad y seguridad), y Windows SmartScreen muestra un aviso (Más información, Ejecutar de todas formas). La firma y la notarización reales necesitan certificados, que deben guardarse en los secretos de GitHub y nunca en el repositorio; consulta las guías de Tauri para [macOS](https://v2.tauri.app/distribute/sign/macos/) y [Windows](https://v2.tauri.app/distribute/sign/windows/).

## Estructura del proyecto

```text
src/         Interfaz de usuario en React + TypeScript
src-tauri/   Núcleo en Rust: redirección de puertos SSH, almacenamiento de perfiles, almacén de credenciales del sistema, bandeja
.github/     Flujos de trabajo de CI y de publicación
```
