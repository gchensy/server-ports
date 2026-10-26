//! What every tunnel shares with the rest of the app, and how it reports back: the latest runtime
//! state and the recent log lines of each profile. Every change is also pushed to the
//! [`EventSink`].

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

use crate::events::EventSink;
use crate::interaction::Interaction;
use crate::known_hosts::KnownHosts;
use crate::model::{
    ConnState, Event, ForwardRuntime, LogLevel, LogLine, ProfileId, ProfileRuntime,
};
use crate::secrets::SecretStore;
use crate::util::now_ms;

const LOG_CAPACITY: usize = 300;

/// The services every tunnel works with.
#[derive(Clone)]
pub struct Env {
    pub known_hosts: Arc<KnownHosts>,
    pub secrets: Arc<dyn SecretStore>,
    pub interaction: Arc<Interaction>,
    pub board: Arc<StatusBoard>,
}

impl Env {
    pub fn new(
        known_hosts: KnownHosts,
        secrets: impl SecretStore,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            known_hosts: Arc::new(known_hosts),
            secrets: Arc::new(secrets),
            interaction: Arc::new(Interaction::new(sink.clone())),
            board: Arc::new(StatusBoard::new(sink)),
        }
    }
}

/// The latest state and log of every profile.
pub struct StatusBoard {
    sink: Arc<dyn EventSink>,
    runtime: Mutex<HashMap<ProfileId, ProfileRuntime>>,
    logs: Mutex<HashMap<ProfileId, VecDeque<LogLine>>>,
    next_seq: AtomicU64,
}

impl StatusBoard {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            sink,
            runtime: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
            next_seq: AtomicU64::new(1),
        }
    }

    /// Applies `change` to the profile's runtime state and announces the result if it differs.
    /// The sink is called with no lock held, so it may call back into this type; that also means
    /// two announcements can overtake each other, which is what `rev` is for.
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
            entry.rev = before.rev + 1;
            entry.clone()
        };
        self.sink.emit(Event::State(snapshot));
    }

    pub fn log(&self, id: &str, level: LogLevel, message: String) {
        let line = LogLine {
            seq: self.next_seq.fetch_add(1, Relaxed),
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

    /// The recent lines of every profile, oldest first.
    pub fn all_logs(&self) -> Vec<LogLine> {
        let mut all: Vec<LogLine> = self
            .logs
            .lock()
            .unwrap()
            .values()
            .flatten()
            .cloned()
            .collect();
        all.sort_by_key(|line| line.seq);
        all
    }

    /// Drops what is remembered about a deleted profile.
    pub fn forget(&self, id: &str) {
        self.runtime.lock().unwrap().remove(id);
        self.logs.lock().unwrap().remove(id);
    }
}

/// The [`StatusBoard`] pinned to one profile: what a running tunnel uses to say what it is doing.
#[derive(Clone)]
pub struct Reporter {
    board: Arc<StatusBoard>,
    profile_id: ProfileId,
}

impl Reporter {
    pub fn new(board: Arc<StatusBoard>, profile_id: &str) -> Self {
        Self {
            board,
            profile_id: profile_id.to_string(),
        }
    }

    pub fn set_conn(&self, conn: ConnState) {
        self.board.update(&self.profile_id, |runtime| {
            runtime.connected_at = match conn {
                ConnState::Connected => runtime.connected_at.or(Some(now_ms())),
                _ => None,
            };
            runtime.conn = conn;
        });
    }

    pub fn set_forwards(&self, forwards: Vec<ForwardRuntime>) {
        self.board.update(&self.profile_id, |runtime| {
            runtime.forwards = forwards;
        });
    }

    pub fn info(&self, message: impl Into<String>) {
        self.board
            .log(&self.profile_id, LogLevel::Info, message.into());
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.board
            .log(&self.profile_id, LogLevel::Warn, message.into());
    }

    pub fn error(&self, message: impl Into<String>) {
        self.board
            .log(&self.profile_id, LogLevel::Error, message.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ChannelSink;

    fn board() -> (
        Arc<StatusBoard>,
        tokio::sync::mpsc::UnboundedReceiver<Event>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Arc::new(StatusBoard::new(Arc::new(ChannelSink(tx)))), rx)
    }

    #[test]
    fn unchanged_state_is_not_announced_twice() {
        let (board, mut rx) = board();
        let reporter = Reporter::new(board.clone(), "p");
        reporter.set_conn(ConnState::Connecting);
        reporter.set_conn(ConnState::Connecting);
        reporter.set_conn(ConnState::Connected);
        let mut states = Vec::new();
        while let Ok(Event::State(s)) = rx.try_recv() {
            states.push((s.conn, s.rev));
        }
        assert_eq!(
            states,
            [(ConnState::Connecting, 1), (ConnState::Connected, 2)],
            "every announced change gets the next revision"
        );
        assert!(board.runtime_of("p").unwrap().connected_at.is_some());
        reporter.set_conn(ConnState::Idle);
        assert!(board.runtime_of("p").unwrap().connected_at.is_none());
    }

    #[test]
    fn the_log_keeps_only_the_most_recent_lines() {
        let (board, _rx) = board();
        let reporter = Reporter::new(board.clone(), "p");
        for i in 0..(LOG_CAPACITY + 10) {
            reporter.info(format!("line {i}"));
        }
        let lines = board.logs("p");
        assert_eq!(lines.len(), LOG_CAPACITY);
        assert_eq!(lines[0].message, "line 10");
        board.forget("p");
        assert!(board.logs("p").is_empty());
    }

    #[test]
    fn log_lines_are_numbered_across_profiles() {
        let (board, _rx) = board();
        Reporter::new(board.clone(), "a").info("first");
        Reporter::new(board.clone(), "b").warn("second");
        Reporter::new(board.clone(), "a").error("third");
        let all = board.all_logs();
        let messages: Vec<_> = all.iter().map(|l| l.message.as_str()).collect();
        assert_eq!(messages, ["first", "second", "third"]);
        assert!(all.windows(2).all(|pair| pair[0].seq < pair[1].seq));
    }
}
