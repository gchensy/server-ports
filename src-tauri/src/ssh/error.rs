//! Everything that can go wrong with a tunnel, classified by what the user can do about it.
//!
//! The Display text is raw technical detail (it ends up under a localised headline in the UI);
//! [`TunnelError::code`] is the stable identifier the UI keys its messages on.

use crate::model::ErrorInfo;

#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    /// The server could not be reached (DNS, refused, unreachable...).
    #[error("{0}")]
    Network(String),
    #[error("timed out {0}")]
    Timeout(&'static str),
    /// Wrong password, key not authorised, ...
    #[error("{0}")]
    Auth(String),
    #[error("the host key was not trusted")]
    HostKeyRejected,
    #[error("host key changed: on file {known}, presented {presented}")]
    HostKeyChanged { known: String, presented: String },
    #[error("the server presented a host certificate, which is not supported")]
    HostCertificate,
    /// The private key file is missing or unreadable.
    #[error("{0}")]
    KeyFile(String),
    #[error("{0}")]
    Agent(String),
    /// The user declined to enter a secret, or pressed "disconnect".
    #[error("cancelled")]
    Cancelled,
    /// None of the local ports could be opened.
    #[error("{0}")]
    NoListeners(String),
    /// No algorithm in common with the server.
    #[error("{0}")]
    Negotiation(String),
    #[error("{0}")]
    Ssh(String),
    /// An established session went away.
    #[error("{0}")]
    Closed(String),
    /// A bug: the tunnel task panicked.
    #[error("{0}")]
    Internal(String),
}

impl TunnelError {
    pub fn code(&self) -> &'static str {
        match self {
            TunnelError::Network(_) => "network",
            TunnelError::Timeout(_) => "timeout",
            TunnelError::Auth(_) => "auth",
            TunnelError::HostKeyRejected => "hostKeyRejected",
            TunnelError::HostKeyChanged { .. } => "hostKeyChanged",
            TunnelError::HostCertificate => "hostCertificate",
            TunnelError::KeyFile(_) => "keyFile",
            TunnelError::Agent(_) => "agent",
            TunnelError::Cancelled => "cancelled",
            TunnelError::NoListeners(_) => "noListeners",
            TunnelError::Negotiation(_) => "negotiation",
            TunnelError::Ssh(_) => "ssh",
            TunnelError::Closed(_) => "closed",
            TunnelError::Internal(_) => "internal",
        }
    }

    /// Whether trying again later can help. Anything that needs the user to change something
    /// (credentials, key, trust decision, a port that is taken) is not retried.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            TunnelError::Network(_)
                | TunnelError::Timeout(_)
                | TunnelError::Ssh(_)
                | TunnelError::Closed(_)
        )
    }

    pub fn info(&self) -> ErrorInfo {
        ErrorInfo {
            code: self.code().to_string(),
            detail: self.to_string(),
        }
    }
}

impl From<russh::Error> for TunnelError {
    fn from(error: russh::Error) -> Self {
        use russh::Error as E;
        let text = error.to_string();
        match error {
            E::ConnectionTimeout => TunnelError::Timeout("connecting"),
            E::Elapsed(_) => TunnelError::Timeout("waiting for the server"),
            E::KeepaliveTimeout | E::InactivityTimeout => TunnelError::Closed(text),
            E::HUP | E::Disconnect | E::SendError | E::RecvError => TunnelError::Closed(text),
            E::IO(io) => TunnelError::Network(io.to_string()),
            E::NoCommonAlgo { .. } | E::KexInit | E::Kex | E::UnknownAlgo => {
                TunnelError::Negotiation(text)
            }
            E::Keys(keys) => TunnelError::KeyFile(keys.to_string()),
            _ => TunnelError::Ssh(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_transient_failures_are_retried() {
        assert!(TunnelError::Network("refused".into()).retryable());
        assert!(TunnelError::Timeout("connecting").retryable());
        assert!(TunnelError::Closed("reset".into()).retryable());
        assert!(!TunnelError::Auth("denied".into()).retryable());
        assert!(!TunnelError::HostKeyRejected.retryable());
        assert!(
            !TunnelError::HostKeyChanged {
                known: "a".into(),
                presented: "b".into()
            }
            .retryable()
        );
        assert!(!TunnelError::NoListeners("in use".into()).retryable());
        assert!(!TunnelError::Negotiation("none in common".into()).retryable());
        assert!(!TunnelError::Cancelled.retryable());
        assert!(!TunnelError::Internal("panic".into()).retryable());
    }

    #[test]
    fn io_errors_are_network_errors() {
        let e: TunnelError =
            russh::Error::IO(std::io::Error::from(std::io::ErrorKind::ConnectionRefused)).into();
        assert_eq!(e.code(), "network");
    }
}
