use super::balance::TokenBalance;
use crate::Result;
use candid::Principal;

#[enum_dispatch::enum_dispatch]
#[allow(async_fn_in_trait)]
pub trait TokenOperations {
    async fn load_balance(&self, user_principal: Principal) -> Result<TokenBalance>;
    async fn deduct_balance(&self, user_principal: Principal, amount: u64) -> Result<u64>;
    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()>;
}
