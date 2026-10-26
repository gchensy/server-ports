//! The tunnel engine: keeps SSH sessions alive and the local listeners that feed them.
//!
//! Objects, from the outside in:
//! - [`TunnelManager`] — the facade; one `Tunnel` per running profile.
//! - `Tunnel` — one profile's lifetime: connect, serve, back off, reconnect.
//! - `Session` — one authenticated SSH connection and the `Forwarder`s riding on it.
//! - `Forwarder` — one local listener; each accepted socket becomes a `direct-tcpip` channel.
//!
//! Around them: [`Env`] is what every tunnel shares (host keys, secrets, questions to the user,
//! status board), `ClientHandler` answers russh's callbacks and `Authenticator` logs in with a
//! password, a key file or ssh-agent.

mod auth;
mod env;
mod error;
mod forward;
mod handler;
mod manager;
mod session;
mod tunnel;

#[cfg(test)]
mod tests;

pub use env::{Env, Reporter, StatusBoard};
pub use error::TunnelError;
pub use manager::TunnelManager;
