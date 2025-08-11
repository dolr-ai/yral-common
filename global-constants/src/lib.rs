use std::ops::Range;

// Creator commission
pub const CREATOR_COMMISSION_PERCENT: u64 = 10;

// Withdraw limits
pub const MIN_WITHDRAWAL_PER_TXN_SATS: u64 = 50;
pub const MAX_WITHDRAWAL_PER_TXN_SATS: u64 = 60;
pub const MAX_WITHDRAWAL_PER_DAY_SATS: u64 = 60;

// Reward limit
pub const NEW_USER_SIGNUP_REWARD_SATS: u64 = 30;
pub const REFERRAL_REWARD_SATS: u64 = 5;
pub const SATS_AIRDROP_LIMIT_RANGE_SATS: Range<u64> = 25..30;
pub const AIRDROP_REWARD_SATS: u64 = 1000;
pub const AIRDROP_REWARD_PER_DAY_SATS: u64 = 10000;

// For workers - SATS as a service
pub const MAX_CKBTC_TREASURY_PER_DAY_PER_USER: u64 = 500;
pub const MAX_CREDITED_PER_DAY_PER_USER_SATS: u64 = 1_000_000;
pub const MAX_DEDUCTED_PER_DAY_PER_USER_SATS: u64 = 100_000;

// Coin state control
pub const BET_COIN_ENABLED_STATES: [CoinState; 2] = [CoinState::C1, CoinState::C5];
pub const DEFAULT_BET_COIN_FOR_LOGGED_IN: CoinState = CoinState::C5;
pub const DEFAULT_BET_COIN_FOR_LOGGED_OUT: CoinState = CoinState::C1;
pub const MAX_BET_AMOUNT_SATS: u64 = 5; // CoinState::C5 is 5

pub const USERNAME_MAX_LEN: usize = 15;

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

impl CoinState {
    pub fn to_cents(&self) -> u64 {
        match self {
            CoinState::C1 => 1,
            CoinState::C5 => 5,
            CoinState::C10 => 10,
            CoinState::C20 => 20,
            CoinState::C50 => 50,
            CoinState::C100 => 100,
            CoinState::C200 => 200,
        }
    }
    pub fn from_cents(cents: u64) -> CoinState {
        match cents {
            1 => CoinState::C1,
            5 => CoinState::C5,
            10 => CoinState::C10,
            20 => CoinState::C20,
            50 => CoinState::C50,
            100 => CoinState::C100,
            200 => CoinState::C200,
            _ => DEFAULT_BET_COIN_FOR_LOGGED_OUT,
        }
    }
    pub fn wrapping_next(self) -> Self {
        BET_COIN_ENABLED_STATES
            .iter()
            .position(|&x| x == self)
            .map(|idx| BET_COIN_ENABLED_STATES[(idx + 1) % BET_COIN_ENABLED_STATES.len()])
            .unwrap_or(DEFAULT_BET_COIN_FOR_LOGGED_OUT)
    }

    pub fn wrapping_prev(self) -> Self {
        BET_COIN_ENABLED_STATES
            .iter()
            .position(|&x| x == self)
            .map(|idx| {
                BET_COIN_ENABLED_STATES
                    [(idx + BET_COIN_ENABLED_STATES.len() - 1) % BET_COIN_ENABLED_STATES.len()]
            })
            .unwrap_or(DEFAULT_BET_COIN_FOR_LOGGED_OUT)
    }
}

// VIDEOGEN COMMON CONSTANTS

pub const GLOBAL_VIDEOGEN_RATE_LIMIT_PER_DAY: u64 = 1000; // Total generations allowed per day

// Video Generation Model Costs in USD cents
pub const VEO3_COST_USD_CENTS: u64 = 50; // $0.5
pub const VEO3_FAST_COST_USD_CENTS: u64 = 50; // $0.5
pub const RAY2FLASH_COST_USD_CENTS: u64 = 50; // $0.5
pub const SEEDANCE_COST_USD_CENTS: u64 = 50; // $0.5
pub const INTTEST_COST_USD_CENTS: u64 = 0; // Free for testing

// Token conversion ratios for video generation
// Based on: $0.5 (50 cents) = 500 sats = 100 dolr
pub const VIDEOGEN_USD_CENTS_TO_SATS: u64 = 10; // 1 cent = 10 sats
pub const VIDEOGEN_USD_CENTS_TO_DOLR_E8S: u64 = 200_000_000; // 1 cent = 2 dolr = 2×10^8 e8s
