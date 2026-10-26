//! Shared bookkeeping for all tunnels: the latest runtime state and the recent log lines of each
//! profile. Every change is also pushed to the [`EventSink`].

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::events::EventSink;
use crate::interaction::Interaction;
use crate::known_hosts::KnownHosts;
use crate::model::{
    ConnState, Event, ForwardRuntime, LogLevel, LogLine, ProfileId, ProfileRuntime,
};
use crate::util::now_ms;

const LOG_CAPACITY: usize = 300;

/// Everything a running tunnel needs from the outside.
pub struct Env {
    pub rt: tokio::runtime::Handle,
    pub known_hosts: Arc<KnownHosts>,
    pub interaction: Arc<Interaction>,
    pub shared: Arc<Shared>,
}

pub struct Shared {
    sink: Arc<dyn EventSink>,
    runtime: Mutex<HashMap<ProfileId, ProfileRuntime>>,
    logs: Mutex<HashMap<ProfileId, VecDeque<LogLine>>>,
}

impl Shared {
    pub fn new(sink: Arc<dyn EventSink>) -> Arc<Self> {
        Arc::new(Self {
            sink,
            runtime: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
        })
    }

    /// Applies `change` to the profile's runtime state and announces the result if it differs.
    /// The sink is called with no lock held, so it may call back into this type.
    pub fn update(&self, id: &str, change: impl FnOnce(&mut ProfileRuntime)) {
        let snapshot = {
            let mut map = self.runtime.lock().unwrap();
            let entry = map
                .entry(id.to_string())
                .or_insert_with(|| ProfileRuntime::idle(id));
            let before = entry.clone();
            change(entry);
            if *entry == before {
                return;
            }
            entry.clone()
        };
        self.sink.emit(Event::State(snapshot));
    }

    pub fn log(&self, id: &str, level: LogLevel, message: String) {
        let line = LogLine {
            profile_id: id.to_string(),
            ts: now_ms(),
            level,
            message,
        };
        {
            let mut logs = self.logs.lock().unwrap();
            let lines = logs.entry(id.to_string()).or_default();
            if lines.len() == LOG_CAPACITY {
                lines.pop_front();
            }
            lines.push_back(line.clone());
        }
        self.sink.emit(Event::Log(line));
    }

    pub fn snapshot(&self) -> Vec<ProfileRuntime> {
        self.runtime.lock().unwrap().values().cloned().collect()
    }

    pub fn runtime_of(&self, id: &str) -> Option<ProfileRuntime> {
        self.runtime.lock().unwrap().get(id).cloned()
    }

    pub fn logs(&self, id: &str) -> Vec<LogLine> {
        self.logs
            .lock()
            .unwrap()
            .get(id)
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Drops what is remembered about a deleted profile.
    pub fn forget(&self, id: &str) {
        self.runtime.lock().unwrap().remove(id);
        self.logs.lock().unwrap().remove(id);
    }
}

/// A [`Shared`] pinned to one profile: what a running tunnel uses to say what it is doing.
#[derive(Clone)]
pub struct Reporter {
    shared: Arc<Shared>,
    profile_id: ProfileId,
}

impl Reporter {
    pub fn new(shared: Arc<Shared>, profile_id: &str) -> Self {
        Self {
            shared,
            profile_id: profile_id.to_string(),
        }
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn set_conn(&self, conn: ConnState) {
        self.shared.update(&self.profile_id, |runtime| {
            runtime.connected_at = match conn {
                ConnState::Connected => runtime.connected_at.or(Some(now_ms())),
                _ => None,
            };
            runtime.conn = conn;
        });
    }

    pub fn set_forwards(&self, forwards: Vec<ForwardRuntime>) {
        self.shared.update(&self.profile_id, |runtime| {
            runtime.forwards = forwards;
        });
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        self.shared.log(&self.profile_id, level, message.into());
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ChannelSink;

    #[test]
    fn unchanged_state_is_not_announced_twice() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let shared = Shared::new(Arc::new(ChannelSink(tx)));
        let rep = Reporter::new(shared.clone(), "p");
        rep.set_conn(ConnState::Connecting);
        rep.set_conn(ConnState::Connecting);
        rep.set_conn(ConnState::Connected);
        let mut states = Vec::new();
        while let Ok(Event::State(s)) = rx.try_recv() {
            states.push(s.conn);
        }
        assert_eq!(states, [ConnState::Connecting, ConnState::Connected]);
        let runtime = shared.runtime_of("p").unwrap();
        assert!(runtime.connected_at.is_some());
        rep.set_conn(ConnState::Idle);
        assert!(shared.runtime_of("p").unwrap().connected_at.is_none());
    }

    #[test]
    fn the_log_keeps_only_the_most_recent_lines() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let shared = Shared::new(Arc::new(ChannelSink(tx)));
        let rep = Reporter::new(shared.clone(), "p");
        for i in 0..(LOG_CAPACITY + 10) {
            rep.info(format!("line {i}"));
        }
        let lines = shared.logs("p");
        assert_eq!(lines.len(), LOG_CAPACITY);
        assert_eq!(lines[0].message, "line 10");
        shared.forget("p");
        assert!(shared.logs("p").is_empty());
    }
}
