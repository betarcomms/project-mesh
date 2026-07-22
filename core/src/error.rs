use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("envelope rejected: {0}")]
    EnvelopeRejected(&'static str),
    #[error("malformed wire data: {0}")]
    Malformed(&'static str),
    #[error("handshake error: {0}")]
    Handshake(String),
    #[error("ratchet error: {0}")]
    Ratchet(&'static str),
    #[error("crypto error: {0}")]
    Crypto(&'static str),
}

pub type Result<T> = std::result::Result<T, MeshError>;
