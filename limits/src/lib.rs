use std::ops::Range;

// Withdraw limits
pub const MIN_WITHDRAWAL_PER_TXN: u64 = 50;
pub const MAX_WITHDRAWAL_PER_TXN: u64 = 100;
pub const MAX_WITHDRAWAL_PER_DAY: u64 = 10000;

// Reward limit
pub const NEW_USER_SIGNUP_REWARD: u64 = 25;
pub const REFERRAL_REWARD: u64 = 5;
pub const SATS_AIRDROP_LIMIT_RANGE: Range<u64> = 25..35;
pub const AIRDROP_REWARD: u64 = 1000;
pub const AIRDROP_REWARD_PER_DAY: u64 = 10000;

// Coin state control
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoinState {
    C1,
    C5,
    C10,
    C20,
    C50,
    C100,
    C200,
}

pub const BET_COIN_ENABLED_STATES: [CoinState; 3] = [CoinState::C1, CoinState::C5, CoinState::C10];
pub const DEFAULT_BET_COIN_STATE: CoinState = CoinState::C10;
pub const MAX_BET_AMOUNT: usize = 10;
pub const TEST_VAR_REMOVE_THIS: usize = 0;
