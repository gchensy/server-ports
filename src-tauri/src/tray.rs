//! The tray icon: a click brings the window back, the menu switches each profile on or off.

use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tokio::sync::Notify;

use crate::app::show_main_window;
use crate::backend::Backend;
use crate::model::ConnState;

const ICON_ID: &str = "main";
const SHOW: &str = "show";
const NO_PROFILES: &str = "no-profiles";
const CONNECT_ALL: &str = "connect-all";
const DISCONNECT_ALL: &str = "disconnect-all";
const QUIT: &str = "quit";
const PROFILE_PREFIX: &str = "profile:";

/// The words of the menu. The UI owns the translations and sends them over; until it does the
/// menu is in English.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Labels {
    pub show: String,
    pub no_profiles: String,
    pub connect_all: String,
    pub disconnect_all: String,
    pub quit: String,
    /// Shown when the pointer rests on the icon; `{n}` stands for the number of connected profiles.
    pub tooltip: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            show: "Show Server Ports".into(),
            no_profiles: "No servers yet".into(),
            connect_all: "Connect all".into(),
            disconnect_all: "Disconnect all".into(),
            quit: "Quit".into(),
            tooltip: "Server Ports: {n} connected".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Row {
    id: String,
    name: String,
    /// Connecting, connected or waiting to reconnect: a click on the entry would disconnect.
    active: bool,
}

/// Everything the menu shows. It is redrawn only when this changes, which is rare compared to the
/// traffic statistics that make the engine report all the time.
#[derive(Debug, Clone, PartialEq)]
struct View {
    labels: Labels,
    rows: Vec<Row>,
    connected: usize,
}

pub struct Tray {
    app: AppHandle,
    icon: TrayIcon,
    backend: Arc<Backend>,
    labels: Mutex<Labels>,
    drawn: Mutex<Option<View>>,
    dirty: Arc<Notify>,
}

impl Tray {
    /// Creates the icon and starts keeping it up to date. `dirty` is poked whenever what the menu
    /// shows may have changed.
    pub fn install(
        app: &AppHandle,
        backend: Arc<Backend>,
        dirty: Arc<Notify>,
    ) -> tauri::Result<Arc<Self>> {
        let icon = TrayIconBuilder::with_id(ICON_ID)
            .icon(app.default_window_icon().cloned().ok_or_else(|| {
                tauri::Error::InvalidIcon(std::io::Error::other("the app has no icon"))
            })?)
            .show_menu_on_left_click(false)
            .on_menu_event(on_menu_event)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main_window(tray.app_handle());
                }
            })
            .build(app)?;
        let tray = Arc::new(Self {
            app: app.clone(),
            icon,
            backend,
            labels: Mutex::new(Labels::default()),
            drawn: Mutex::new(None),
            dirty,
        });
        tray.refresh();
        tray.clone().redraw_when_asked();
        Ok(tray)
    }

    /// Something the menu shows may have changed.
    pub fn invalidate(&self) {
        self.dirty.notify_one();
    }

    /// Draws again even if nothing changed: the system has already flipped the check mark of the
    /// entry that was clicked, which is wrong if the click did nothing in the end.
    fn redraw(&self) {
        *self.drawn.lock().unwrap() = None;
        self.invalidate();
    }

    pub fn set_labels(&self, labels: Labels) {
        *self.labels.lock().unwrap() = labels;
        self.invalidate();
    }

    /// Redraws on the main thread, where menus belong, however many changes came in meanwhile.
    fn redraw_when_asked(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            loop {
                self.dirty.notified().await;
                let tray = self.clone();
                if self.app.run_on_main_thread(move || tray.refresh()).is_err() {
                    return;
                }
            }
        });
    }

    fn refresh(&self) {
        let view = self.view();
        if self.drawn.lock().unwrap().as_ref() == Some(&view) {
            return;
        }
        match self.draw(&view) {
            Ok(()) => *self.drawn.lock().unwrap() = Some(view),
            Err(e) => log::warn!("could not update the tray menu: {e}"),
        }
    }

    fn view(&self) -> View {
        let mut connected = 0;
        let rows = self
            .backend
            .profiles()
            .into_iter()
            .map(|profile| {
                let conn = self.backend.runtime_of(&profile.id).map(|r| r.conn);
                connected += usize::from(conn == Some(ConnState::Connected));
                Row {
                    active: conn.is_some_and(|conn| conn.is_active()),
                    id: profile.id,
                    name: profile.name,
                }
            })
            .collect();
        View {
            labels: self.labels.lock().unwrap().clone(),
            rows,
            connected,
        }
    }

    fn draw(&self, view: &View) -> tauri::Result<()> {
        self.icon.set_menu(Some(self.menu(view)?))?;
        let tooltip = view
            .labels
            .tooltip
            .replace("{n}", &view.connected.to_string());
        self.icon.set_tooltip(Some(tooltip))
    }

    fn menu(&self, view: &View) -> tauri::Result<Menu<Wry>> {
        let app = &self.app;
        let labels = &view.labels;
        let item = |id: &str, text: &str, enabled: bool| {
            MenuItem::with_id(app, id, text, enabled, None::<&str>)
        };
        let menu = Menu::new(app)?;
        menu.append(&item(SHOW, &labels.show, true)?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        if view.rows.is_empty() {
            menu.append(&item(NO_PROFILES, &labels.no_profiles, false)?)?;
        }
        for row in &view.rows {
            let id = format!("{PROFILE_PREFIX}{}", row.id);
            // A lone `&` would make the next letter a keyboard mnemonic.
            let name = row.name.replace('&', "&&");
            menu.append(&CheckMenuItem::with_id(
                app,
                id,
                name,
                true,
                row.active,
                None::<&str>,
            )?)?;
        }
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        let any_idle = view.rows.iter().any(|row| !row.active);
        let any_active = view.rows.iter().any(|row| row.active);
        menu.append(&item(CONNECT_ALL, &labels.connect_all, any_idle)?)?;
        menu.append(&item(DISCONNECT_ALL, &labels.disconnect_all, any_active)?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        menu.append(&item(QUIT, &labels.quit, true)?)?;
        Ok(menu)
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let backend = app.state::<Arc<Backend>>().inner().clone();
    let tray = app.state::<Arc<Tray>>().inner().clone();
    let id = event.id().as_ref();
    if let Some(profile_id) = id.strip_prefix(PROFILE_PREFIX) {
        let profile_id = profile_id.to_string();
        tauri::async_runtime::spawn(async move {
            // What the click does depends on the state now, not on the check mark that was flipped.
            let active = backend
                .runtime_of(&profile_id)
                .is_some_and(|runtime| runtime.conn.is_active());
            if active {
                backend.disconnect(&profile_id).await;
            } else if let Err(e) = backend.connect(&profile_id).await {
                log::warn!("could not connect from the tray: {e}");
            }
            tray.redraw();
        });
        return;
    }
    match id {
        SHOW => show_main_window(app),
        QUIT => app.exit(0),
        CONNECT_ALL => {
            tauri::async_runtime::spawn(async move { backend.connect_all().await });
        }
        DISCONNECT_ALL => {
            tauri::async_runtime::spawn(async move { backend.disconnect_all().await });
        }
        _ => tray.redraw(),
    }
}
