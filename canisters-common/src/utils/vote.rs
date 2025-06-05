use candid::{CandidType, Principal};
use canisters_client::individual_user_template::BettingStatus;
use hon_worker_common::{GameInfo, GameInfoReq};
use serde::{Deserialize, Serialize};
use web_time::Duration;
use yral_identity::{ic_agent::sign_message, msg_builder::Message, Signature};

use crate::{consts::CENTS_IN_E6S, Canisters, Error, HonError, Result};

use super::time::current_epoch;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum VoteOutcome {
    Won(u64),
    Draw(u64),
    Lost,
    AwaitingResult,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, CandidType)]
pub enum VoteKind {
    Hot,
    Not,
}

impl From<VoteKind> for hon_worker_common::HotOrNot {
    fn from(value: VoteKind) -> Self {
        match value {
            VoteKind::Hot => Self::Hot,
            VoteKind::Not => Self::Not,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, CandidType)]
pub struct HonBetArg {
    pub bet_amount: u64,
    pub post_id: u64,
    pub bet_direction: VoteKind,
    pub post_canister_id: Principal,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VerifiableHonBetReq {
    pub sender: Principal,
    pub signature: Signature,
    pub args: HonBetArg,
}

pub fn verifiable_hon_bet_message(args: HonBetArg) -> Message {
    Message::default()
        .method_name("place_hon_bet_worker_req".into())
        .args((args,))
        .expect("Place bet request should serialize")
}

impl VerifiableHonBetReq {
    pub fn new(sender: &impl ic_agent::Identity, args: HonBetArg) -> yral_identity::Result<Self> {
        let msg = verifiable_hon_bet_message(args);
        let signature = sign_message(sender, msg)?;

        Ok(Self {
            sender: sender.sender().expect("signing was succesful"),
            args,
            signature,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VoteDetails {
    pub outcome: VoteOutcome,
    pub post_id: u64,
    pub canister_id: Principal,
    pub vote_kind: VoteKind,
    pub vote_amount: u64,
    placed_at: Duration,
    slot_id: u8,
}

impl VoteDetails {
    pub fn reward(&self) -> Option<u64> {
        match self.outcome {
            VoteOutcome::Won(w) => Some(w),
            VoteOutcome::Draw(w) => Some(w),
            VoteOutcome::Lost => None,
            VoteOutcome::AwaitingResult => None,
        }
    }

    pub fn vote_duration(&self) -> Duration {
        // Vote duration + 5 minute overhead
        Duration::from_secs(((self.slot_id as u64) * 60 * 60) + 5 * 60)
    }

    pub fn end_time(&self, post_creation_time: Duration) -> Duration {
        post_creation_time + self.vote_duration()
    }

    pub fn time_remaining(&self, post_creation_time: Duration) -> Duration {
        let end_time = self.end_time(post_creation_time);
        end_time.saturating_sub(current_epoch())
    }
}

impl Canisters<true> {
    /// Places a vote on a post via cloudflare. The vote amount must be in cents e0s
    pub async fn vote_with_cents_on_post_via_cloudflare(
        &self,
        cloudflare_url: reqwest::Url,
        vote_amount: u64,
        bet_direction: VoteKind,
        post_id: u64,
        post_canister_id: Principal,
    ) -> Result<BettingStatus> {
        let req = VerifiableHonBetReq::new(
            self.identity(),
            HonBetArg {
                bet_amount: vote_amount * CENTS_IN_E6S,
                post_id,
                bet_direction,
                post_canister_id,
            },
        )?;

        let url = cloudflare_url.join("/place_hot_or_not_bet")?;

        let client = reqwest::Client::new();
        let betting_status: BettingStatus =
            client.post(url).json(&req).send().await?.json().await?;

        Ok(betting_status)
    }

    pub async fn fetch_game_with_sats_info(
        &self,
        cloudflare_url: reqwest::Url,
        request: GameInfoReq,
    ) -> Result<Option<GameInfo>> {
        let path = format!("/game_info/{}", self.user_principal());
        let url = cloudflare_url.join(&path)?;

        let client = reqwest::Client::new();
        let res = client.post(url).json(&request).send().await?;

        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(Error::Hon(HonError::Backend(err)));
        }

        let info = res.json().await?;

        Ok(info)
    }
}
