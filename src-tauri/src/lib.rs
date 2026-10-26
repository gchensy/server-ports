mod app;
pub mod backend;
mod commands;
pub mod events;
pub mod interaction;
pub mod known_hosts;
pub mod model;
pub mod secrets;
pub mod ssh;
pub mod store;
#[cfg(test)]
mod testing;
mod tray;
pub mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run()
}
