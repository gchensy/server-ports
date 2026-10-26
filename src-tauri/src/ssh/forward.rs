//! One local listener of a profile. Every connection it accepts becomes a `direct-tcpip` channel
//! on the SSH session, so the target sees it coming from the server itself.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use russh::ChannelOpenFailure;
use russh::client::Handle;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, copy_bidirectional_with_sizes};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::env::Reporter;
use super::handler::ClientHandler;
use crate::model::{ErrorInfo, Forward, ForwardRuntime, ForwardState};

/// The SSH session that carries the connections.
pub type Link = Arc<Handle<ClientHandler>>;

/// Matches the SSH packet size, so one read fills one packet.
const COPY_BUFFER: usize = 32 * 1024;
/// A browser retrying a dead target would otherwise fill the log with the same line.
const WARN_EVERY: Duration = Duration::from_secs(10);

/// Counters shared by the accept loop, the connection tasks and the status ticker.
#[derive(Default)]
struct Stats {
    active: AtomicU32,
    total: AtomicU64,
    up: AtomicU64,
    down: AtomicU64,
}

/// Counts a connection as active for as long as it lives.
struct Live(Arc<Stats>);

impl Live {
    fn start(stats: &Arc<Stats>) -> Self {
        stats.active.fetch_add(1, Relaxed);
        stats.total.fetch_add(1, Relaxed);
        Self(stats.clone())
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Relaxed);
    }
}

/// The local end of a connection, counting what passes through it: what is read from it travels
/// up to the server, what is written to it came down from the server.
struct Metered {
    socket: TcpStream,
    stats: Arc<Stats>,
}

impl AsyncRead for Metered {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.socket).poll_read(cx, buf);
        if poll.is_ready() {
            let read = buf.filled().len() - before;
            self.stats.up.fetch_add(read as u64, Relaxed);
        }
        poll
    }
}

impl AsyncWrite for Metered {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.socket).poll_write(cx, buf);
        if let Poll::Ready(Ok(written)) = &poll {
            self.stats.down.fetch_add(*written as u64, Relaxed);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_shutdown(cx)
    }
}

/// Where a mapping's connections go; shared by the accept loop and every connection task.
#[derive(Clone)]
struct Route {
    forward: Forward,
    link: Link,
    stats: Arc<Stats>,
    reporter: Reporter,
    cancel: CancellationToken,
    warned: Arc<Mutex<Option<Instant>>>,
}

impl Route {
    async fn accept_from(self, listener: TcpListener) {
        loop {
            let accepted = tokio::select! {
                _ = self.cancel.cancelled() => return,
                accepted = listener.accept() => accepted,
            };
            match accepted {
                Ok((socket, peer)) => {
                    let _ = socket.set_nodelay(true);
                    tokio::spawn(self.clone().carry(socket, peer));
                }
                Err(e) => {
                    // Typically out of file descriptors: do not spin.
                    self.warn(format!("accept failed: {e}"));
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }

    async fn carry(self, socket: TcpStream, peer: SocketAddr) {
        let _live = Live::start(&self.stats);
        let Forward {
            remote_host,
            remote_port,
            ..
        } = &self.forward;
        let opened = tokio::select! {
            _ = self.cancel.cancelled() => return,
            opened = self.link.channel_open_direct_tcpip(
                remote_host.as_str(),
                u32::from(*remote_port),
                peer.ip().to_string(),
                u32::from(peer.port()),
            ) => opened,
        };
        let channel = match opened {
            Ok(channel) => channel,
            Err(e) => {
                self.warn(open_failure(&e, remote_host, *remote_port));
                return;
            }
        };
        let mut local = Metered {
            socket,
            stats: self.stats.clone(),
        };
        let mut remote = channel.into_stream();
        tokio::select! {
            _ = self.cancel.cancelled() => {}
            _ = copy_bidirectional_with_sizes(&mut local, &mut remote, COPY_BUFFER, COPY_BUFFER) => {}
        }
    }

    fn warn(&self, text: String) {
        {
            let mut last = self.warned.lock().unwrap();
            if last.is_some_and(|at| at.elapsed() < WARN_EVERY) {
                return;
            }
            *last = Some(Instant::now());
        }
        self.reporter.warn(format!(
            "{}:{}: {text}",
            self.forward.bind_host, self.forward.local_port
        ));
    }
}

fn open_failure(error: &russh::Error, host: &str, port: u16) -> String {
    match error {
        russh::Error::ChannelOpenFailure(ChannelOpenFailure::ConnectFailed) => {
            format!("the server could not connect to {host}:{port}")
        }
        russh::Error::ChannelOpenFailure(ChannelOpenFailure::AdministrativelyProhibited) => {
            "the server does not allow port forwarding (AllowTcpForwarding)".to_string()
        }
        other => other.to_string(),
    }
}

fn bind_error(error: &io::Error) -> ErrorInfo {
    let code = match error.kind() {
        io::ErrorKind::AddrInUse => "addrInUse",
        io::ErrorKind::PermissionDenied => "permissionDenied",
        io::ErrorKind::AddrNotAvailable => "addrNotAvailable",
        _ => "bind",
    };
    ErrorInfo {
        code: code.to_string(),
        detail: error.to_string(),
    }
}

async fn bind(forward: &Forward) -> io::Result<TcpListener> {
    let ip: IpAddr = forward
        .bind_host
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    TcpListener::bind((ip, forward.local_port)).await
}

pub struct Forwarder {
    forward: Forward,
    stats: Arc<Stats>,
    state: ForwardState,
    /// The accept loop; it owns the listener, so the port is free once it has ended.
    accepting: Option<JoinHandle<()>>,
}

impl Forwarder {
    /// Opens the local port and starts accepting. A port that cannot be opened does not fail the
    /// session: the forwarder reports it and the other mappings keep working.
    pub async fn start(
        forward: Forward,
        link: Link,
        reporter: Reporter,
        cancel: CancellationToken,
    ) -> Self {
        let stats = Arc::new(Stats::default());
        let (state, accepting) = match bind(&forward).await {
            Ok(listener) => {
                reporter.info(format!(
                    "forwarding {}:{} to {}:{} on the server",
                    forward.bind_host, forward.local_port, forward.remote_host, forward.remote_port
                ));
                let route = Route {
                    forward: forward.clone(),
                    link,
                    stats: stats.clone(),
                    reporter,
                    cancel,
                    warned: Arc::default(),
                };
                let accepting = tokio::spawn(route.accept_from(listener));
                (ForwardState::Listening, Some(accepting))
            }
            Err(e) => {
                reporter.warn(format!(
                    "cannot listen on {}:{}: {e}",
                    forward.bind_host, forward.local_port
                ));
                (
                    ForwardState::Failed {
                        error: bind_error(&e),
                    },
                    None,
                )
            }
        };
        Self {
            forward,
            stats,
            state,
            accepting,
        }
    }

    /// Waits until the local port is closed. Call it after the cancellation token has fired.
    pub async fn closed(self) {
        if let Some(accepting) = self.accepting {
            let _ = accepting.await;
        }
    }

    pub fn is_listening(&self) -> bool {
        self.state == ForwardState::Listening
    }

    /// Why the local port could not be opened, if it could not.
    pub fn failure(&self) -> Option<String> {
        match &self.state {
            ForwardState::Failed { error } => Some(format!(
                "{}:{}: {}",
                self.forward.bind_host, self.forward.local_port, error.detail
            )),
            _ => None,
        }
    }

    pub fn runtime(&self) -> ForwardRuntime {
        ForwardRuntime {
            active: self.stats.active.load(Relaxed),
            total: self.stats.total.load(Relaxed),
            bytes_up: self.stats.up.load(Relaxed),
            bytes_down: self.stats.down.load(Relaxed),
            ..ForwardRuntime::new(&self.forward.id, self.state.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_failures_get_a_code_the_ui_can_localise() {
        let code = |kind| bind_error(&io::Error::from(kind)).code;
        assert_eq!(code(io::ErrorKind::AddrInUse), "addrInUse");
        assert_eq!(code(io::ErrorKind::PermissionDenied), "permissionDenied");
        assert_eq!(code(io::ErrorKind::AddrNotAvailable), "addrNotAvailable");
        assert_eq!(code(io::ErrorKind::Other), "bind");
    }
}
