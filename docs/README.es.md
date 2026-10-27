<div align="center">

# Server Ports

**Asigna puertos de servidores remotos a tu equipo local a través de SSH.**

[English](../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | Español | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Estado: desarrollo inicial.** La primera versión es la v0.1.0. El motor de túneles y la interfaz ya existen, y la integración continua (CI) compila, prueba y empaqueta la aplicación en Linux, macOS y Windows, pero hasta ahora solo se ha instalado y probado el `.deb` de Linux (en Ubuntu): los demás instaladores no se han probado y ninguno está firmado.

Server Ports es una pequeña aplicación de escritorio multiplataforma (Windows, macOS y Linux) que mantiene activas tus redirecciones de puertos SSH. Elige un servidor, indica los puertos que necesitas y aparecerán en `localhost`. Es lo mismo que `ssh -L`, sin tener que andar con la terminal.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="La ventana de Server Ports: la lista de servidores a la izquierda y, a la derecha, el servidor seleccionado con su conexión y dos reenvíos de puertos" width="860">
  </picture>
</p>

## Características

- **Perfiles de servidor**: un perfil por cada servidor SSH, con tantas asignaciones de puertos como necesites.
- **Importación desde tu configuración SSH**: elige servidores de `~/.ssh/config` y cópialos a Server Ports de una vez, en el diálogo «Nuevo servidor» o desde la pantalla de inicio. Se traen el nombre del host, el usuario, el puerto, el archivo de clave (`IdentityFile`) y las redirecciones (`LocalForward`), incluidos los valores heredados de los bloques `Host *` y de los archivos `Include`. Es una copia única: tu configuración SSH solo se lee, nunca se modifica.
- **Tres formas de iniciar sesión**: contraseña, archivo de clave privada (con frase de contraseña opcional) o ssh-agent (agente de OpenSSH o Pageant).
- **Los secretos no van al archivo de configuración**: si lo pides, las contraseñas y las frases de contraseña de las claves se guardan en el Llavero de macOS, el Administrador de credenciales de Windows o el Secret Service de Linux; si no, solo se conservan en memoria mientras el servidor está conectado.
- **Verificación de la clave del host**: la clave de un servidor que no habías visto antes se muestra con su huella digital y solo se acepta si tú lo confirmas; si una clave cambia, se avisa.
- **Privado por defecto**: las redirecciones escuchan en `127.0.0.1`, de modo que solo tu propio equipo puede acceder a ellas.
- **Reconexión automática**: las conexiones caídas se reintentan solas, con una espera cada vez mayor (de 1 s hasta 30 s), y un perfil puede conectarse en cuanto arranca la aplicación.
- **Estado en tiempo real**: estado de la conexión de cada servidor, estado de escucha de cada asignación, conexiones abiertas, contadores de tráfico y un pequeño gráfico de rendimiento.
- **Vive en la bandeja del sistema**: al cerrar la ventana, la aplicación se oculta en la bandeja del sistema en lugar de salir (se puede desactivar), y el menú de la bandeja activa o desactiva cada servidor. La aplicación también puede iniciarse al iniciar sesión.
- **Tema claro y oscuro**, con la interfaz en nueve idiomas: inglés, chino simplificado y tradicional, japonés, coreano, español, francés, alemán y ruso. Sigue el idioma del sistema hasta que elijas uno.
- **Ligera**: creada con Tauri 2, con un núcleo en Rust y la vista web del sistema, sin navegador incluido.

## Instalación

Descarga el instalador para tu sistema desde la [página de Releases](https://github.com/gchensy/server-ports/releases) y ejecútalo:

- **Windows**: el instalador `.exe`, o el `.msi`. Los instaladores todavía no están firmados, así que Windows SmartScreen muestra un aviso la primera vez: elige Más información y después Ejecutar de todas formas.
- **macOS**: el `.dmg` (`aarch64` para Apple silicon, `x64` para Intel). Arrastra Server Ports a Aplicaciones. La aplicación solo lleva una firma ad hoc, así que la primera vez ábrela con clic derecho y Abrir, o permítela en Ajustes del Sistema, Privacidad y seguridad.
- **Linux**: el `.deb` (Debian, Ubuntu), el `.rpm` (Fedora, openSUSE) o el `.AppImage`. Por ejemplo:

```bash
sudo apt install ./Server*.deb     # Debian, Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

En GNOME, el icono de la bandeja necesita la extensión AppIndicator (consulta «Limitaciones conocidas» más abajo). La aplicación no se actualiza sola: para actualizar, instala la versión nueva sobre la anterior.

## Primeros pasos

Las capturas muestran la interfaz en inglés; los nombres de los botones del texto son los de la interfaz en español.

### 1. Añade un servidor

Haz clic en **Nuevo servidor** y rellena el formulario:

- **Servidor**: el host (un nombre o una dirección IP), el puerto SSH, el usuario y, si quieres, un nombre para la lista.
- **Inicio de sesión**: una **Contraseña**, un **Archivo de clave** privada (con **Examinar…** lo eliges) o el **Agente SSH**.
- **Reenvío de puertos**: escribe en **Escuchar en** el puerto que quieres usar en tu propio equipo; el puerto del servidor al que se reenvía (**Reenviar a**) lo sigue, y puedes cambiarlo. Añade un reenvío por cada puerto que necesites, con una etiqueta si quieres. **Avanzado** muestra la **Dirección local** en la que escucha un puerto: `127.0.0.1` lo limita a tu propio equipo.
- **Opciones**: **Reconectar automáticamente** cuando se cae la conexión y **Conectar al iniciar la aplicación**.

<p align="center">
  <img src="images/new-server.png" alt="El cuadro de diálogo «Nuevo servidor», ya rellenado" width="440">
</p>

**Guardar y conectar** guarda el servidor y se conecta al instante; **Guardar** solo lo guarda. Después, los botones del lápiz y de la papelera, arriba a la derecha de un servidor, sirven para editarlo y eliminarlo.

¿Ya tienes tus servidores en `~/.ssh/config`? Cambia el cuadro de diálogo a **Desde config. SSH**, marca los servidores que quieras y haz clic en **Importar**. El host, el usuario, el puerto, el archivo de clave y los puertos de `LocalForward` vienen con ellos; tu configuración de SSH solo se lee, nunca se modifica.

<p align="center">
  <img src="images/import-ssh-config.png" alt="El cuadro de diálogo «Nuevo servidor» con la lista de servidores encontrados en la configuración de SSH" width="520">
</p>

### 2. Conéctate

Activa el servidor en la lista, o ábrelo y haz clic en **Conectar**. La primera vez que te conectas a un servidor, Server Ports muestra el tipo de clave y la huella digital que ha presentado y te pregunta si confías en él. Compara la huella con la que esperas (en el servidor, `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` la imprime) y haz clic en **Confiar y continuar**, o en **No conectar** si no es la que esperas. La clave se recuerda a partir de entonces y, si alguna vez cambia, la conexión se detiene y se te avisa.

<p align="center">
  <img src="images/trust-server.png" alt="La pregunta «¿Confiar en este servidor?», con el tipo de clave y la huella digital" width="380">
</p>

Si el servidor pide una contraseña, o tu clave tiene una frase de contraseña, Server Ports te la pedirá a continuación. Marca **Recordar en el llavero del sistema** para guardarla en el almacén de credenciales de tu sistema; si no, solo se conserva en memoria mientras el servidor está conectado.

### 3. Usa los puertos

Cuando el servidor está conectado, sus reenvíos de puertos muestran un punto verde y los puertos están abiertos en tu equipo: abre `http://127.0.0.1:8080` en el navegador o apunta cualquier herramienta al puerto local, como un cliente de base de datos a `127.0.0.1:5432`.

- Los dos botones a la derecha de un reenvío abren su dirección en el navegador y la copian.
- Debajo de la dirección ves un pequeño gráfico de rendimiento, los datos enviados y recibidos, y cuántas conexiones hay abiertas ahora y en total.
- La pestaña **Actividad** muestra lo que ha pasado en este servidor (conexión, reconexión, errores): mira ahí primero si algo no funciona.
- Si la conexión se cae, Server Ports se reconecta solo (salvo que lo hayas desactivado para ese servidor), esperando un poco más cada vez, hasta 30 s. **Reintentar ahora** se salta la espera.

### 4. Déjala en marcha

Cierra la ventana y los túneles siguen funcionando en la bandeja del sistema. Haz clic en el icono de la bandeja, o elige **Mostrar Server Ports** en su menú, para recuperar la ventana; iniciar la aplicación por segunda vez hace lo mismo. El menú de la bandeja también activa o desactiva cada servidor, conecta o desconecta todos a la vez y tiene **Salir**.

Abre **Configuración**, abajo a la izquierda de la ventana, para elegir el idioma y la apariencia, iniciar la aplicación al iniciar sesión, o hacer que se cierre junto con su ventana en lugar de quedarse en la bandeja. **Salir de Server Ports** cierra la aplicación y todos sus túneles.

<p align="center">
  <img src="images/settings.png" alt="El cuadro de diálogo «Configuración»" width="360">
</p>

## Cómo funciona

Cada asignación redirige un puerto de tu equipo a una dirección a la que el servidor SSH puede llegar:

```text
127.0.0.1:8080 (tu equipo)  ──SSH cifrado──►  127.0.0.1:80 (el servidor)
```

Equivale a `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. El destino se resuelve en el servidor, así que `127.0.0.1` es el propio servidor, y también sirve cualquier otro host al que el servidor pueda llegar.

## Limitaciones conocidas

- Todavía no admite el inicio de sesión interactivo con teclado (doble factor), los saltos a través de otros servidores (`ProxyJump`) ni los certificados de host.
- Solo redirecciones locales (`ssh -L`); las remotas (`-R`) y las SOCKS (`-D`) no están soportadas.
- La importación desde `~/.ssh/config` es una copia única; los cambios posteriores en el archivo no se recogen. Los hosts que necesitan `ProxyJump` o `ProxyCommand` aparecen en la lista, pero todavía no se pueden importar; los bloques `Match` se omiten y las líneas `RemoteForward` y `DynamicForward` se ignoran. Un host sin `LocalForward` abre el formulario para que elijas sus puertos.
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
docs/        Traducciones de este README y las capturas de pantalla que muestran
.github/     Flujos de trabajo de CI y de publicación
```
