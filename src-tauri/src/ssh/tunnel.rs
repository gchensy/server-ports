//! One profile's life: connect, serve, and after a failure back off and try again.

use std::time::{Duration, Instant};

use tokio::task::{JoinError, JoinHandle};
use tokio_util::sync::CancellationToken;

use super::auth::KeptSecrets;
use super::env::{Env, Reporter};
use super::error::TunnelError;
use super::session::Session;
use crate::model::{ConnState, ForwardRuntime, ForwardState, Profile};
use crate::util::now_ms;

/// A session that lasted this long was healthy: the next failure starts the back-off over.
const STABLE_AFTER: Duration = Duration::from_secs(30);

/// A running tunnel. Dropping it does not stop it; call [`Tunnel::stop`].
pub struct Tunnel {
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl Tunnel {
    /// Must be called inside a tokio runtime.
    pub fn start(profile: Profile, env: Env) -> Self {
        let cancel = CancellationToken::new();
        let reporter = Reporter::new(env.board.clone(), &profile.id);
        let supervisor = Supervisor {
            profile,
            env,
            reporter: reporter.clone(),
            cancel: cancel.clone(),
            kept: KeptSecrets::default(),
            attempt: 0,
        };
        let task = tokio::spawn(async move {
            // A panic in the engine must not leave the profile looking alive.
            if let Err(e) = tokio::spawn(supervisor.run()).await {
                reporter.error("internal error, please report a bug");
                let error = TunnelError::Internal(panic_message(e));
                reporter.set_conn(ConnState::Failed {
                    error: error.info(),
                });
            }
        });
        Self { cancel, task }
    }

    /// Asks the tunnel to stop without waiting for it.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Stops the tunnel and waits until it has reported `Idle`.
    pub async fn stop(self) {
        self.cancel.cancel();
        let _ = self.task.await;
    }

    /// The tunnel gave up or was stopped; it is not going to connect again by itself.
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }
}

fn panic_message(error: JoinError) -> String {
    let Ok(payload) = error.try_into_panic() else {
        return "the tunnel task was aborted".to_string();
    };
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// 1 s, 2 s, 4 s ... capped at 30 s.
fn backoff(attempt: u32) -> Duration {
    Duration::from_secs((1u64 << attempt.saturating_sub(1).min(5)).min(30))
}

struct Supervisor {
    profile: Profile,
    env: Env,
    reporter: Reporter,
    cancel: CancellationToken,
    /// Typed secrets that worked, so that a reconnect can log in without a question.
    kept: KeptSecrets,
    /// Consecutive failed attempts.
    attempt: u32,
}

impl Supervisor {
    async fn run(mut self) {
        loop {
            self.reporter.set_conn(ConnState::Connecting);
            let mut up_since = None;
            let outcome = self.connect_and_serve(&mut up_since).await;
            self.reporter.set_forwards(self.pending());
            let error = match outcome {
                Ok(()) => break,
                Err(_) if self.cancel.is_cancelled() => break,
                Err(error) => error,
            };
            self.reporter.error(error.to_string());
            if !error.retryable() || !self.profile.auto_reconnect {
                self.reporter.set_conn(ConnState::Failed {
                    error: error.info(),
                });
                return;
            }
            if up_since.is_some_and(|since: Instant| since.elapsed() >= STABLE_AFTER) {
                self.attempt = 0;
            }
            self.attempt += 1;
            let delay = backoff(self.attempt);
            self.reporter
                .info(format!("reconnecting in {} s", delay.as_secs()));
            self.reporter.set_conn(ConnState::Reconnecting {
                attempt: self.attempt,
                retry_at: now_ms() + delay.as_millis() as u64,
                error: error.info(),
            });
            tokio::select! {
                _ = self.cancel.cancelled() => break,
                _ = tokio::time::sleep(delay) => {}
            }
        }
        self.reporter.info("disconnected");
        self.reporter.set_forwards(Vec::new());
        self.reporter.set_conn(ConnState::Idle);
    }

    /// One session from the first packet to the last. `Ok` means the tunnel was stopped.
    async fn connect_and_serve(&self, up_since: &mut Option<Instant>) -> Result<(), TunnelError> {
        self.reporter.set_forwards(self.pending());
        let mut session = Session::connect(
            &self.profile,
            &self.env,
            &self.reporter,
            &self.cancel,
            &self.kept,
        )
        .await?;
        let outcome = async {
            session.open_forwards(&self.profile.forwards).await?;
            *up_since = Some(Instant::now());
            self.reporter.set_conn(ConnState::Connected);
            session.serve().await
        }
        .await;
        session.close().await;
        outcome
    }

    /// The mappings as they look before their ports are open.
    fn pending(&self) -> Vec<ForwardRuntime> {
        self.profile
            .forwards
            .iter()
            .map(|forward| ForwardRuntime::new(&forward.id, ForwardState::Pending))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_up_to_thirty_seconds() {
        let secs: Vec<_> = (1..=8).map(|n| backoff(n).as_secs()).collect();
        assert_eq!(secs, [1, 2, 4, 8, 16, 30, 30, 30]);
        assert_eq!(backoff(0).as_secs(), 1);
        assert_eq!(backoff(u32::MAX).as_secs(), 30);
    }
}
