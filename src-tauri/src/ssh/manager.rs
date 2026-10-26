//! The engine's front door: starts and stops the tunnels of profiles.

use std::collections::HashMap;

use tokio::sync::Mutex;

use super::env::Env;
use super::tunnel::Tunnel;
use crate::model::{Profile, ProfileId};

pub struct TunnelManager {
    env: Env,
    /// Held across a stop, so that a start right after it cannot overlap the old tunnel's last
    /// state updates.
    running: Mutex<HashMap<ProfileId, Tunnel>>,
}

impl TunnelManager {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            running: Mutex::new(HashMap::new()),
        }
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Starts connecting. Does nothing if the profile's tunnel is already running (a tunnel that
    /// gave up counts as not running, so starting it again is a retry).
    pub async fn start(&self, profile: Profile) {
        let mut running = self.running.lock().await;
        if running.get(&profile.id).is_some_and(|t| !t.is_finished()) {
            return;
        }
        let id = profile.id.clone();
        running.insert(id, Tunnel::start(profile, self.env.clone()));
    }

    /// Replaces a running tunnel by one with the given settings; does nothing if the profile is not
    /// running.
    pub async fn restart_if_running(&self, profile: Profile) {
        let mut running = self.running.lock().await;
        if running.get(&profile.id).is_none_or(Tunnel::is_finished) {
            return;
        }
        if let Some(old) = running.remove(&profile.id) {
            old.stop().await;
        }
        let id = profile.id.clone();
        running.insert(id, Tunnel::start(profile, self.env.clone()));
    }

    /// Disconnects and waits until the profile has reported `Idle`.
    pub async fn stop(&self, id: &str) {
        let mut running = self.running.lock().await;
        if let Some(tunnel) = running.remove(id) {
            tunnel.stop().await;
        }
    }

    /// Stops every tunnel, in parallel. Used when the app quits.
    pub async fn stop_all(&self) {
        let mut running = self.running.lock().await;
        let tunnels: Vec<Tunnel> = running.drain().map(|(_, tunnel)| tunnel).collect();
        tunnels.iter().for_each(Tunnel::cancel);
        for tunnel in tunnels {
            tunnel.stop().await;
        }
    }

    pub async fn is_running(&self, id: &str) -> bool {
        let running = self.running.lock().await;
        running.get(id).is_some_and(|t| !t.is_finished())
    }
}
