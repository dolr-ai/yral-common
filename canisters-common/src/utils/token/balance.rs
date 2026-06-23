use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Sub, SubAssign},
    str::FromStr,
};

use candid::Nat;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenBalance {
    pub e8s: Nat,
    decimals: u8,
}

impl TokenBalance {
    pub fn new(e8s: Nat, decimals: u8) -> Self {
        Self { e8s, decimals }
    }

    pub fn parse(token_str: &str, decimals: u8) -> Result<Self, rust_decimal::Error> {
        let scale_factor = 10u64.pow(decimals.into());
        let tokens = (Decimal::from_str(token_str)? * Decimal::new(scale_factor as i64, 0)).floor();
        let e8s = Nat::from_str(&tokens.to_string()).unwrap();
        Ok(Self::new(e8s, decimals))
    }

    // Human friendly token amount
    pub fn humanize(&self) -> String {
        (self.e8s.clone() / 10u64.pow(self.decimals as u32))
            .to_string()
            .replace("_", ",")
    }

    // Humanize the amount, but as a float
    pub fn humanize_float(&self) -> String {
        let tokens = Decimal::from_str(&self.e8s.0.to_str_radix(10)).unwrap()
            / Decimal::new(10i64.pow(self.decimals as u32), 0);
        tokens.to_string()
    }

    // Humanize the amount, but as a truncated float to specified decimal points (dp)
    pub fn humanize_float_truncate_to_dp(&self, dp: u32) -> String {
        let tokens = Decimal::from_str(&self.e8s.0.to_str_radix(10)).unwrap()
            / Decimal::new(10i64.pow(self.decimals as u32), 0);
        tokens
            .round_dp_with_strategy(dp, RoundingStrategy::ToZero)
            .to_string()
    }

    // Returns number of tokens(not e8s)
    pub fn to_tokens(&self) -> String {
        let tokens = self.e8s.clone() / Nat::from(10u64.pow(self.decimals as u32));
        tokens.0.to_str_radix(10)
    }
}

impl From<TokenBalance> for Nat {
    fn from(value: TokenBalance) -> Nat {
        value.e8s
    }
}

impl<T> Add<T> for TokenBalance
where
    Nat: Add<T, Output = Nat>,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        Self {
            e8s: self.e8s + other,
            decimals: self.decimals,
        }
    }
}

impl<T> AddAssign<T> for TokenBalance
where
    Nat: AddAssign<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.e8s += rhs;
    }
}

impl<T> PartialEq<T> for TokenBalance
where
    Nat: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        self.e8s.eq(other)
    }
}

impl<T> PartialOrd<T> for TokenBalance
where
    Nat: PartialOrd<T>,
{
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.e8s.partial_cmp(other)
    }
}

impl<T> Sub<T> for TokenBalance
where
    Nat: Sub<T, Output = Nat>,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            e8s: self.e8s - rhs,
            decimals: self.decimals,
        }
    }
}

impl<T> SubAssign<T> for TokenBalance
where
    Nat: SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: T) {
        self.e8s -= rhs;
    }
}

impl Sub for TokenBalance {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            e8s: self.e8s - rhs.e8s,
            decimals: self.decimals,
        }
    }
}

impl SubAssign<TokenBalance> for TokenBalance {
    fn sub_assign(&mut self, rhs: TokenBalance) {
        self.e8s -= rhs.e8s;
    }
}

impl PartialEq for TokenBalance {
    fn eq(&self, other: &Self) -> bool {
        self.e8s.eq(&other.e8s)
    }
}

impl PartialOrd for TokenBalance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.e8s.partial_cmp(&other.e8s)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TokenBalanceOrClaiming(Option<TokenBalance>);

impl TokenBalanceOrClaiming {
    pub fn new(balance: TokenBalance) -> Self {
        Self(Some(balance))
    }

    pub fn claiming() -> Self {
        Self(None)
    }

    pub fn is_claiming(&self) -> bool {
        self.0.is_none()
    }

    pub fn humanize(&self) -> String {
        self.0
            .as_ref()
            .map(|b| b.humanize())
            .unwrap_or_else(|| "Processing".to_string())
    }

    pub fn humanize_float(&self) -> String {
        self.map_balance_ref(|b| b.humanize_float())
            .unwrap_or_else(|| "Processing".to_string())
    }

    pub fn humanize_float_truncate_to_dp(&self, dp: u32) -> String {
        self.map_balance_ref(|b| b.humanize_float_truncate_to_dp(dp))
            .unwrap_or_else(|| "Processing".to_string())
    }

    pub fn map_balance<T>(self, f: impl FnOnce(TokenBalance) -> T) -> Option<T> {
        self.0.map(f)
    }

    pub fn map_balance_ref<T>(&self, f: impl FnOnce(&TokenBalance) -> T) -> Option<T> {
        self.0.as_ref().map(f)
    }
}
