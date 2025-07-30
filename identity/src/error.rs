use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("candid ser/de error {0}")]
    Candid(#[from] candid::Error),
    #[error("signature missing")]
    MissingSignature,
    #[error("signing failed {0}")]
    Signing(String),
    #[error("signature verification failed {0}")]
    SignatureVerification(String),
    #[error("invalid message {0}")]
    InvalidMessage(String),
    #[error("signature does not match identity")]
    IdentityMismatch,
    #[error("sender not found in identity")]
    SenderNotFound,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
