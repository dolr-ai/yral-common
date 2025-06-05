use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum WorkerError {
    #[error("Invalid Signature")]
    InvalidSignature,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("user has already voted on this post")]
    AlreadyVotedOnPost,
    #[error("post not found")]
    PostNotFound,
    #[error("user does not have sufficient balance")]
    InsufficientFunds,
    #[error("treasury is out of funds")]
    TreasuryOutOfFunds,
    #[error("treasury limit reached, try again tomorrow")]
    TreasuryLimitReached,
    #[error("user was already referred")]
    AlreadyReferred,
    #[error("specified airdropped with negative delta")]
    InvalidAirdropDelta,
}

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum AirdropClaimError {
    #[error("Invalid Signature")]
    InvalidSignature,
}
