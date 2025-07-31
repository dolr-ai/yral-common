use candid::Principal;
use super::balance::TokenBalance;
use crate::Result;

#[enum_dispatch::enum_dispatch]
pub trait TokenOperations {
    async fn load_balance(&self, user_principal: Principal) -> Result<TokenBalance>;
    async fn deduct_balance(&self, user_principal: Principal, amount: u64) -> Result<u64>;
    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()>;
}