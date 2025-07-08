mod error;

pub use error::*;

use candid::{CandidType, Nat, Principal};
use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use yral_identity::{Signature, msg_builder::Message};

pub const WORKER_URL: &str = "https://yral-hot-or-not-stage.go-bazzinga.workers.dev/";
pub type WorkerResponse<T> = Result<T, WorkerError>;

#[derive(Serialize, Deserialize, Clone, Debug, CandidType)]
pub struct ClaimRequest {
    /// User's principal id
    pub user_principal: Principal,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifiableClaimRequest {
    pub sender: Principal,
    pub request: ClaimRequest,
    /// The amount of airdrop to be claimed in sats (e0s)
    pub amount: u64,
    pub signature: Signature,
}

pub fn verifiable_claim_request_message(args: ClaimRequest) -> Message {
    Message::default()
        .method_name("claim_sats_airdrop_request".into())
        .args((args,))
        .expect("Request must serialize")
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SatsBalanceInfo {
    pub balance: BigUint,
    pub airdropped: BigUint,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SatsBalanceInfoV2 {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub balance: BigUint,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub airdropped: BigUint,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug, CandidType)]
pub enum HotOrNot {
    Hot,
    Not,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameResult {
    Win { win_amt: BigUint },
    Loss { lose_amt: BigUint },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameResultV2 {
    Win {
        win_amt: BigUint,
        updated_balance: BigUint,
    },
    Loss {
        lose_amt: BigUint,
        updated_balance: BigUint,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameInfo {
    CreatorReward(BigUint),
    Vote {
        vote_amount: BigUint,
        game_result: GameResult,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameInfoV2 {
    CreatorReward(BigUint),
    Vote {
        vote_amount: BigUint,
        game_result: GameResultV2,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoteRes {
    pub game_result: GameResult,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoteResV2 {
    pub game_result: GameResultV2,
}

#[derive(Serialize, Deserialize, Clone, Debug, CandidType)]
pub struct VoteRequest {
    pub post_canister: Principal,
    pub post_id: u64,
    pub vote_amount: u128,
    pub direction: HotOrNot,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameRes {
    pub post_canister: Principal,
    pub post_id: u64,
    pub game_info: GameInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameResV2 {
    pub post_canister: Principal,
    pub post_id: u64,
    pub game_info: GameInfoV2,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedGamesReq {
    pub page_size: usize,
    pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedGamesRes {
    pub games: Vec<GameRes>,
    pub next: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedGamesResV2 {
    pub games: Vec<GameResV2>,
    pub next: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct GameInfoReq {
    pub post_canister: Principal,
    pub post_id: u64,
}

impl From<(Principal, u64)> for GameInfoReq {
    fn from((post_canister, post_id): (Principal, u64)) -> Self {
        Self {
            post_canister,
            post_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HoNGameVoteReq {
    pub request: VoteRequest,
    /// Sentiment from alloydb
    pub fetched_sentiment: HotOrNot,
    pub post_creator: Option<Principal>,
    pub signature: Signature,
}

pub fn hon_game_vote_msg(request: VoteRequest) -> yral_identity::msg_builder::Message {
    yral_identity::msg_builder::Message::default()
        .method_name("hon_worker_game_vote".into())
        .args((request,))
        .expect("Vote request should serialize")
}

#[cfg(feature = "client")]
pub fn sign_vote_request(
    sender: &impl ic_agent::Identity,
    request: VoteRequest,
) -> yral_identity::Result<Signature> {
    use yral_identity::ic_agent::sign_message;
    let msg = hon_game_vote_msg(request.clone());
    sign_message(sender, msg)
}

#[cfg(feature = "client")]
pub fn sign_claim_request(
    sender: &impl ic_agent::Identity,
    request: ClaimRequest,
) -> yral_identity::Result<Signature> {
    use yral_identity::ic_agent::sign_message;
    let msg = verifiable_claim_request_message(request.clone());
    sign_message(sender, msg)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WithdrawRequest {
    pub receiver: Principal,
    pub amount: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HoNGameWithdrawReq {
    pub request: WithdrawRequest,
    pub signature: Signature,
}

pub fn hon_game_withdraw_msg(request: &WithdrawRequest) -> yral_identity::msg_builder::Message {
    yral_identity::msg_builder::Message::default()
        .method_name("hon_worker_game_withdraw".into())
        .args((request.amount,))
        .expect("Withdraw request should serialize")
}

#[cfg(feature = "client")]
pub fn sign_withdraw_request(
    sender: &impl ic_agent::Identity,
    request: WithdrawRequest,
) -> yral_identity::Result<Signature> {
    use yral_identity::ic_agent::sign_message;
    let msg = hon_game_withdraw_msg(&request);
    sign_message(sender, msg)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReferralItem {
    pub referrer: Principal,
    pub referee: Principal,
    pub amount: u64,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReferralReqWithSignature {
    pub request: ReferralReq,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone, Debug, CandidType)]
pub struct ReferralReq {
    pub referrer: Principal,
    pub referee: Principal,
    pub referee_canister: Principal,
    #[serde(default = "default_referral_amount")]
    pub amount: u64,
}

pub fn default_referral_amount() -> u64 {
    limits::REFERRAL_REWARD
}

pub fn hon_referral_msg(request: ReferralReq) -> yral_identity::msg_builder::Message {
    yral_identity::msg_builder::Message::default()
        .method_name("hon_worker_referral".into())
        .args((request,))
        .expect("Referral request should serialize")
}

#[cfg(feature = "client")]
pub fn sign_referral_request(
    sender: &impl ic_agent::Identity,
    request: ReferralReq,
) -> yral_identity::Result<Signature> {
    use yral_identity::ic_agent::sign_message;
    let msg = hon_referral_msg(request);
    sign_message(sender, msg)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedReferralsReq {
    pub cursor: Option<u64>,
    pub limit: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedReferralsRes {
    pub items: Vec<ReferralItem>,
    pub cursor: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WithdrawalState {
    Value(Nat),
    NeedMoreEarnings(Nat),
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct SatsBalanceUpdateRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub delta: BigInt,
    pub is_airdropped: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct SatsBalanceUpdateRequestV2 {
    pub previous_balance: BigUint,
    #[serde_as(as = "DisplayFromStr")]
    pub delta: BigInt,
    pub is_airdropped: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameResV3 {
    pub publisher_principal: Principal,
    pub post_id: u64,
    pub game_info: GameInfo,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaginatedGamesResV3 {
    pub games: Vec<GameResV3>,
    pub next: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameInfoReqV3 {
    pub publisher_principal: Principal,
    pub post_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, CandidType)]
pub struct VoteRequestV3 {
    pub publisher_principal: Principal,
    pub post_id: u64,
    pub vote_amount: u128,
    pub direction: HotOrNot,
}

#[cfg(feature = "client")]
pub fn sign_vote_request_v3(
    sender: &impl ic_agent::Identity,
    request: VoteRequestV3,
) -> yral_identity::Result<Signature> {
    use yral_identity::ic_agent::sign_message;
    let msg = hon_game_vote_msg_v3(request.clone());
    sign_message(sender, msg)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HoNGameVoteReqV3 {
    pub request: VoteRequestV3,
    /// Sentiment from alloydb
    pub fetched_sentiment: HotOrNot,
    pub post_creator: Option<Principal>,
    pub signature: Signature,
}

pub fn hon_game_vote_msg_v3(request: VoteRequestV3) -> yral_identity::msg_builder::Message {
    yral_identity::msg_builder::Message::default()
        .method_name("hon_worker_game_vote_v3".into())
        .args((request,))
        .expect("Vote request should serialize")
}
