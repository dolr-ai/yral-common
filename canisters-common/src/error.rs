use std::{io, str::FromStr};

use candid::Nat;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PndError {
    #[error("worker didn't return a number: {0}")]
    Parse(<Nat as FromStr>::Err),
    #[error("network error when accessing worker: {0}")]
    Network(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
pub enum HonError {
    #[error("network error when accessing worker: {0}")]
    Network(#[from] reqwest::Error),
    #[error("error accessing game backend: {0}")]
    Backend(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Agent(#[from] ic_agent::AgentError),
    #[error("{0}")]
    Candid(#[from] candid::Error),
    #[error("{0}")]
    Metadata(#[from] yral_metadata_client::Error),
    #[error("error from yral canister: {0}")]
    YralCanister(String),
    #[error("invalid identity: {0}")]
    Identity(String),
    #[error("identity error: {0}")]
    YralIdentity(#[from] identity::Error),
    #[error("failed to get transactions: {0}")]
    GetTransactions(String),
    #[error("failed to parse transaction")]
    ParseTransaction,
    #[error("invalid tip certificate in ledger")]
    TipCertificate,
    #[error("{0}")]
    CborDe(#[from] ciborium::de::Error<io::Error>),
    #[error("{0}")]
    PndError(#[from] PndError),
    #[error("{0}")]
    Hon(#[from] HonError),
    #[error("{0}")]
    Url(#[from] url::ParseError),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
