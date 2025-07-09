use std::ops::Range;

// Withdraw limits
pub const MIN_WITHDRAWAL_PER_TXN_SATS: u64 = 200;
pub const MAX_WITHDRAWAL_PER_TXN_SATS: u64 = 500;
pub const MAX_WITHDRAWAL_PER_DAY_SATS: u64 = 10_000;

// Reward limit
pub const NEW_USER_SIGNUP_REWARD_SATS: u64 = 100;
pub const REFERRAL_REWARD_SATS: u64 = 5;
pub const SATS_AIRDROP_LIMIT_RANGE_SATS: Range<u64> = 50..100;
pub const AIRDROP_REWARD_SATS: u64 = 1000;
pub const AIRDROP_REWARD_PER_DAY_SATS: u64 = 10000;

// Coin state control
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoinState {
    C10,
    C20,
    C50,
    C100,
    C200,
}

impl CoinState {
    pub fn to_cents(&self) -> u64 {
        match self {
            CoinState::C10 => 10,
            CoinState::C20 => 20,
            CoinState::C50 => 50,
            CoinState::C100 => 100,
            CoinState::C200 => 200,
        }
    }
    pub fn from_cents(cents: u64) -> CoinState {
        match cents {
            10 => CoinState::C10,
            20 => CoinState::C20,
            50 => CoinState::C50,
            100 => CoinState::C100,
            200 => CoinState::C200,
            _ => DEFAULT_BET_COIN_STATE,
        }
    }
    pub fn wrapping_next(self) -> Self {
        BET_COIN_ENABLED_STATES.iter()
            .position(|&x| x == self)
            .map(|idx| BET_COIN_ENABLED_STATES[(idx + 1) % BET_COIN_ENABLED_STATES.len()])
            .unwrap_or(DEFAULT_BET_COIN_STATE)
    }

    pub fn wrapping_prev(self) -> Self {
        BET_COIN_ENABLED_STATES.iter()
            .position(|&x| x == self)
            .map(|idx| BET_COIN_ENABLED_STATES[(idx + BET_COIN_ENABLED_STATES.len() - 1) % BET_COIN_ENABLED_STATES.len()])
            .unwrap_or(DEFAULT_BET_COIN_STATE)
    }
}

pub const BET_COIN_ENABLED_STATES: [CoinState; 2] = [CoinState::C10, CoinState::C20];
pub const DEFAULT_BET_COIN_STATE: CoinState = CoinState::C10;
pub const MAX_BET_AMOUNT_SATS: u64 = 20;

// For workers - SATS as a service
pub const MAXIMUM_CKBTC_TREASURY_PER_DAY_PER_USER: u64 = 500;
pub const MAXIMUM_SATS_CREDITED_PER_DAY_PER_USER: u64 = 1_000_000;
pub const MAXIMUM_SATS_DEDUCTED_PER_DAY_PER_USER: u64 = 100_000;
