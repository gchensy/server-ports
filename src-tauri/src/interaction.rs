//! Questions the engine has to put to a human (trust this host key? what is the password?)
//! and the plumbing that carries the answers back.
//!
//! The engine calls [`Interaction::confirm_host_key`] / [`Interaction::request_secret`] and awaits;
//! those emit an [`Event`] for the UI, and the UI answers through `answer_*`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::oneshot;

use crate::events::EventSink;
use crate::model::{Event, HostKeyRequest, SecretRequest};
use crate::util::new_id;

/// SSH servers drop clients that have not authenticated within `LoginGraceTime` (120s by default),
/// so there is no point in waiting for an answer for longer than this.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug)]
pub struct SecretReply {
    pub value: String,
    /// Keep it in the OS credential store.
    pub remember: bool,
}

enum Reply {
    HostKey(oneshot::Sender<bool>),
    Secret(oneshot::Sender<Option<SecretReply>>),
}

struct Pending {
    /// What the UI was asked, kept for a window that opens while the question is still open.
    question: Event,
    reply: Reply,
}

pub struct Interaction {
    sink: Arc<dyn EventSink>,
    pending: Mutex<HashMap<String, Pending>>,
}

/// Withdraws the question when the asking task goes away for any reason (answer, timeout,
/// the user pressing "disconnect" while the dialog is open), so the UI never shows a dead dialog.
struct Withdraw<'a> {
    interaction: &'a Interaction,
    request_id: String,
}

impl Drop for Withdraw<'_> {
    fn drop(&mut self) {
        self.interaction
            .pending
            .lock()
            .unwrap()
            .remove(&self.request_id);
        self.interaction.sink.emit(Event::PromptClosed {
            request_id: std::mem::take(&mut self.request_id),
        });
    }
}

impl Interaction {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            sink,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// `true` only if the user explicitly trusted the key.
    pub async fn confirm_host_key(&self, mut request: HostKeyRequest) -> bool {
        let (tx, rx) = oneshot::channel();
        request.request_id = new_id();
        let _withdraw = Withdraw {
            interaction: self,
            request_id: request.request_id.clone(),
        };
        self.ask(
            request.request_id.clone(),
            Event::HostKey(request),
            Reply::HostKey(tx),
        );
        matches!(tokio::time::timeout(PROMPT_TIMEOUT, rx).await, Ok(Ok(true)))
    }

    /// `None` means the user cancelled (or never answered).
    pub async fn request_secret(&self, mut request: SecretRequest) -> Option<SecretReply> {
        let (tx, rx) = oneshot::channel();
        request.request_id = new_id();
        let _withdraw = Withdraw {
            interaction: self,
            request_id: request.request_id.clone(),
        };
        self.ask(
            request.request_id.clone(),
            Event::Secret(request),
            Reply::Secret(tx),
        );
        match tokio::time::timeout(PROMPT_TIMEOUT, rx).await {
            Ok(Ok(reply)) => reply,
            _ => None,
        }
    }

    /// The questions nobody has answered yet, as the events that announced them.
    pub fn open_questions(&self) -> Vec<Event> {
        let pending = self.pending.lock().unwrap();
        pending.values().map(|p| p.question.clone()).collect()
    }

    /// Returns `false` when the question no longer exists (timed out or withdrawn).
    pub fn answer_host_key(&self, request_id: &str, trust: bool) -> bool {
        match self.take(request_id, |reply| matches!(reply, Reply::HostKey(_))) {
            Some(Reply::HostKey(tx)) => tx.send(trust).is_ok(),
            _ => false,
        }
    }

    /// `reply == None` cancels the prompt.
    pub fn answer_secret(&self, request_id: &str, reply: Option<SecretReply>) -> bool {
        match self.take(request_id, |reply| matches!(reply, Reply::Secret(_))) {
            Some(Reply::Secret(tx)) => tx.send(reply).is_ok(),
            _ => false,
        }
    }

    fn ask(&self, request_id: String, question: Event, reply: Reply) {
        self.pending.lock().unwrap().insert(
            request_id,
            Pending {
                question: question.clone(),
                reply,
            },
        );
        self.sink.emit(question);
    }

    /// Takes the question out if it is still open and `wanted` accepts what kind it is. An answer
    /// of the wrong kind leaves it alone.
    fn take(&self, request_id: &str, wanted: fn(&Reply) -> bool) -> Option<Reply> {
        let mut pending = self.pending.lock().unwrap();
        if !pending.get(request_id).is_some_and(|p| wanted(&p.reply)) {
            return None;
        }
        pending.remove(request_id).map(|p| p.reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ChannelSink;
    use crate::model::SecretKind;

    fn interaction() -> (
        Arc<Interaction>,
        tokio::sync::mpsc::UnboundedReceiver<Event>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Arc::new(Interaction::new(Arc::new(ChannelSink(tx)))), rx)
    }

    fn host_key_request() -> HostKeyRequest {
        HostKeyRequest {
            request_id: String::new(),
            profile_id: "p".into(),
            profile_name: "n".into(),
            host: "h".into(),
            port: 22,
            key_type: "ssh-ed25519".into(),
            fingerprint: "SHA256:x".into(),
            other_known: false,
        }
    }

    #[tokio::test]
    async fn host_key_answer_reaches_the_waiting_task() {
        let (hub, mut events) = interaction();
        let asking = {
            let hub = hub.clone();
            tokio::spawn(async move { hub.confirm_host_key(host_key_request()).await })
        };
        let Some(Event::HostKey(req)) = events.recv().await else {
            panic!("expected a host key request");
        };
        assert!(hub.answer_host_key(&req.request_id, true));
        assert!(asking.await.unwrap());
        // The dialog is told to go away once the question is settled.
        let Some(Event::PromptClosed { request_id }) = events.recv().await else {
            panic!("expected PromptClosed");
        };
        assert_eq!(request_id, req.request_id);
        // A second answer to the same question is ignored.
        assert!(!hub.answer_host_key(&req.request_id, true));
    }

    #[tokio::test]
    async fn dropping_the_asking_task_withdraws_the_question() {
        let (hub, mut events) = interaction();
        let asking = {
            let hub = hub.clone();
            tokio::spawn(async move {
                hub.request_secret(SecretRequest {
                    request_id: String::new(),
                    profile_id: "p".into(),
                    profile_name: "n".into(),
                    kind: SecretKind::Password,
                    retry: false,
                })
                .await
            })
        };
        let Some(Event::Secret(req)) = events.recv().await else {
            panic!("expected a secret request");
        };
        asking.abort();
        let Some(Event::PromptClosed { request_id }) = events.recv().await else {
            panic!("expected PromptClosed after abort");
        };
        assert_eq!(request_id, req.request_id);
        assert!(!hub.answer_secret(&req.request_id, None));
    }

    #[tokio::test]
    async fn a_late_window_can_ask_which_questions_are_open() {
        let (hub, mut events) = interaction();
        let asking = {
            let hub = hub.clone();
            tokio::spawn(async move { hub.confirm_host_key(host_key_request()).await })
        };
        let Some(Event::HostKey(req)) = events.recv().await else {
            panic!("expected a host key request");
        };
        let open = hub.open_questions();
        assert!(matches!(&open[..], [Event::HostKey(r)] if r.request_id == req.request_id));
        assert!(hub.answer_host_key(&req.request_id, false));
        assert!(!asking.await.unwrap());
        assert!(hub.open_questions().is_empty());
    }

    #[tokio::test]
    async fn answering_with_the_wrong_kind_does_not_consume_the_question() {
        let (hub, mut events) = interaction();
        let asking = {
            let hub = hub.clone();
            tokio::spawn(async move { hub.confirm_host_key(host_key_request()).await })
        };
        let Some(Event::HostKey(req)) = events.recv().await else {
            panic!("expected a host key request");
        };
        assert!(!hub.answer_secret(&req.request_id, None));
        assert!(hub.answer_host_key(&req.request_id, false));
        assert!(!asking.await.unwrap());
    }
}
