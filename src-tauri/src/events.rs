//! How the engine reports to the outside world, without knowing who is listening.

use crate::model::Event;

pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: Event);
}

/// Discards everything.
pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) {}
}

/// Forwards events into a tokio channel (used by tests and tools).
pub struct ChannelSink(pub tokio::sync::mpsc::UnboundedSender<Event>);

impl EventSink for ChannelSink {
    fn emit(&self, event: Event) {
        let _ = self.0.send(event);
    }
}
