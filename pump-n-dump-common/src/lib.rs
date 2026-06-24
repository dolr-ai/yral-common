pub mod rest;
pub mod ws;

use candid::Nat;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameDirection {
    Pump,
    Dump,
}

// TODO: individual_user_template removed, needs migration to user_info_service/user_post_service
// The From<GameDirection> for CanisterGameDirection and reverse impls were removed
// because CanisterGameDirection came from individual_user_template::GameDirection.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WithdrawalState {
    Value(Nat),
    NeedMoreEarnings(Nat),
}
