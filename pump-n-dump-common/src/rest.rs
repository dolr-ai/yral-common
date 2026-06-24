use candid::{Nat, Principal};
use identity::{msg_builder::Message, Signature};
use serde::{Deserialize, Serialize};

use crate::GameDirection;

/// ParticipatedGameInfo matching the candid definition (was from individual_user_template.did)
/// TODO: individual_user_template removed, needs migration to user_info_service/user_post_service
#[derive(Serialize, Deserialize, Clone)]
pub struct ParticipatedGameInfo {
    pub game_direction: GameDirection,
    pub reward: Nat,
    pub pumps: u64,
    pub dumps: u64,
    pub token_root: Principal,
}

/// Request for converting GDOLLR to DOLLR
#[derive(Serialize, Deserialize, Clone)]
pub struct ClaimReq {
    // user to send DOLLR to
    pub sender: Principal,
    // amount of DOLLR
    pub amount: Nat,
    // signature asserting the user's consent
    pub signature: Signature,
}

pub fn claim_msg(amount: Nat) -> Message {
    Message::default()
        .method_name("pump_or_dump_worker_claim".into())
        .args((amount,))
        .expect("Claim request should serialize")
}

impl ClaimReq {
    #[cfg(feature = "client")]
    pub fn new(sender: &impl ic_agent::Identity, amount: Nat) -> identity::Result<Self> {
        use identity::ic_agent::sign_message;
        let msg = claim_msg(amount.clone());
        let signature = sign_message(sender, msg)?;

        Ok(Self {
            sender: sender.sender().expect("signing was succesful"),
            amount,
            signature,
        })
    }
}

/// Response for user bets for a given game
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UserBetsResponse {
    pub pumps: u64,
    pub dumps: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BalanceInfoResponse {
    pub net_airdrop_reward: Nat,
    pub balance: Nat,
    pub withdrawable: Nat,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CompletedGameInfo {
    pub pumps: u64,
    pub dumps: u64,
    pub reward: Nat,
    pub token_root: Principal,
    pub outcome: GameDirection,
}

impl From<CompletedGameInfo> for ParticipatedGameInfo {
    fn from(value: CompletedGameInfo) -> Self {
        Self {
            pumps: value.pumps,
            dumps: value.dumps,
            reward: value.reward,
            token_root: value.token_root,
            game_direction: value.outcome,
        }
    }
}

impl From<ParticipatedGameInfo> for CompletedGameInfo {
    fn from(value: ParticipatedGameInfo) -> Self {
        Self {
            pumps: value.pumps,
            dumps: value.dumps,
            reward: value.reward,
            token_root: value.token_root,
            outcome: value.game_direction,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum UncommittedGameInfo {
    Completed(CompletedGameInfo),
    Pending { token_root: Principal },
}

impl UncommittedGameInfo {
    /// Get the game's token root regardless of state
    pub fn token_root(&self) -> Principal {
        match self {
            UncommittedGameInfo::Completed(info) => info.token_root,
            UncommittedGameInfo::Pending { token_root } => *token_root,
        }
    }
}

pub type UncommittedGamesRes = Vec<UncommittedGameInfo>;
