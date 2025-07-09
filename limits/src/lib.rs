use std::ops::Range;

// Withdraw limits
pub const MIN_WITHDRAWAL_PER_TXN: u64 = 200;
pub const MAX_WITHDRAWAL_PER_TXN: u64 = 500;
pub const MAX_WITHDRAWAL_PER_DAY: u64 = 10000;

// Reward limit
pub const NEW_USER_SIGNUP_REWARD: u64 = 100;
pub const REFERRAL_REWARD: u64 = 5;
pub const SATS_AIRDROP_LIMIT_RANGE: Range<u64> = 50..100;
pub const AIRDROP_REWARD: u64 = 1000;
pub const AIRDROP_REWARD_PER_DAY: u64 = 10000;

// Coin state control
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoinState {
    C10,
    C20,
    C50,
    C100,
    C200,
}

pub const BET_COIN_ENABLED_STATES: [CoinState; 2] = [CoinState::C10, CoinState::C20];
pub const DEFAULT_BET_COIN_STATE: CoinState = CoinState::C10;
pub const MAX_BET_AMOUNT_SATS: u64 = 20;

// Treasury
pub const MAXIMUM_CKBTC_TREASURY_PER_DAY_PER_USER: u64 = 500;
pub const MAXIMUM_SATS_CREDITED_PER_DAY_PER_USER: u64 = 1_000_000;
pub const MAXIMUM_SATS_DEDUCTED_PER_DAY_PER_USER: u64 = 100_000;
