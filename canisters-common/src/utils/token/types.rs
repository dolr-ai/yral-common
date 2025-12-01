use candid::{Nat, Principal};
use hon_worker_common::SatsBalanceUpdateRequestV2;
use num_bigint::{BigInt, BigUint, Sign};
use reqwest::Client;
use url::Url;

use super::balance::TokenBalance;
use super::operations::TokenOperations;
use crate::{consts::DOLR_AI_LEDGER_CANISTER, error::Error, Result};
use canisters_client::{
    dedup_index::Result_,
    ic::{self, USER_INFO_SERVICE_ID},
    individual_user_template::Ok,
    sns_ledger::{self, Account as LedgerAccount},
    user_info_service::{Result5, Result_ as UserInfoResult, SubscriptionPlan, UserInfoService},
};

// ckBTC transfer types - no longer needed as we're using direct IC transfers

#[derive(Clone)]
pub struct SatsOperations {
    jwt_token: Option<String>,
    client: Client,
}

impl SatsOperations {
    pub fn new(jwt_token: Option<String>) -> Self {
        Self {
            jwt_token,
            client: Client::new(),
        }
    }
}

impl TokenOperations for SatsOperations {
    async fn load_balance(&self, user_principal: Principal) -> Result<TokenBalance> {
        let url: Url = hon_worker_common::WORKER_URL.parse().unwrap();
        let balance_url = url
            .join(&format!("/balance/{user_principal}"))
            .expect("Url to be valid");

        let res: hon_worker_common::SatsBalanceInfo = self
            .client
            .get(balance_url)
            .send()
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?
            .json()
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        Ok(TokenBalance::new(res.balance.into(), 0))
    }

    async fn deduct_balance(&self, user_principal: Principal, amount: u64) -> Result<u64> {
        let jwt_token = self.jwt_token.as_ref().ok_or_else(|| {
            Error::YralCanister("JWT token required for deduct operation".to_string())
        })?;

        // First, load the current balance
        let current_balance = self.load_balance(user_principal).await?;
        let previous_balance = BigUint::from(current_balance.e8s);

        // Create negative delta for deduction
        let delta = BigInt::from_biguint(Sign::Minus, BigUint::from(amount));

        let url: Url = hon_worker_common::WORKER_URL.parse().unwrap();
        let deduct_url = url
            .join(&format!("/v2/update_balance/{user_principal}"))
            .expect("Url to be valid");

        let worker_req = SatsBalanceUpdateRequestV2 {
            previous_balance,
            delta,
            is_airdropped: false,
        };

        let res = self
            .client
            .post(deduct_url)
            .bearer_auth(jwt_token)
            .json(&worker_req)
            .send()
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        if res.status().is_success() {
            Ok(amount)
        } else {
            let error_text = res
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(Error::YralCanister(format!(
                "Failed to deduct balance: {error_text}"
            )))
        }
    }

    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()> {
        let jwt_token = self.jwt_token.as_ref().ok_or_else(|| {
            Error::YralCanister("JWT token required for add operation".to_string())
        })?;

        // First, load the current balance
        let current_balance = self.load_balance(user_principal).await?;
        let previous_balance = BigUint::from(current_balance.e8s);

        // Create positive delta for addition
        let delta = BigInt::from(amount);

        let url: Url = hon_worker_common::WORKER_URL.parse().unwrap();
        let add_url = url
            .join(&format!("/v2/update_balance/{user_principal}"))
            .expect("Url to be valid");

        let worker_req = SatsBalanceUpdateRequestV2 {
            previous_balance,
            delta,
            is_airdropped: false,
        };

        let res = self
            .client
            .post(add_url)
            .bearer_auth(jwt_token)
            .json(&worker_req)
            .send()
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        if res.status().is_success() {
            Ok(())
        } else {
            let error_text = res
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(Error::YralCanister(format!(
                "Failed to add balance: {error_text}"
            )))
        }
    }
}

#[derive(Clone)]
pub struct DolrOperations {
    admin_agent: ic_agent::Agent,
    user_agent: Option<ic_agent::Agent>,
}

impl DolrOperations {
    pub fn new(admin_agent: ic_agent::Agent) -> Self {
        Self {
            admin_agent,
            user_agent: None,
        }
    }

    pub fn with_user_agent(admin_agent: ic_agent::Agent, user_agent: ic_agent::Agent) -> Self {
        Self {
            admin_agent,
            user_agent: Some(user_agent),
        }
    }
}

impl TokenOperations for DolrOperations {
    async fn load_balance(&self, user_principal: Principal) -> Result<TokenBalance> {
        let ledger_id = Principal::from_text(DOLR_AI_LEDGER_CANISTER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        // Use user agent if available, otherwise admin agent
        let agent = self.user_agent.as_ref().unwrap_or(&self.admin_agent);
        let ledger = sns_ledger::SnsLedger(ledger_id, agent);

        let balance = ledger
            .icrc_1_balance_of(LedgerAccount {
                owner: user_principal,
                subaccount: None,
            })
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        Ok(TokenBalance::new(balance, 8))
    }

    async fn deduct_balance(&self, user_principal: Principal, amount: u64) -> Result<u64> {
        let ledger_id = Principal::from_text(DOLR_AI_LEDGER_CANISTER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        // Get the admin principal (destination for transfers)
        let admin_principal = self
            .admin_agent
            .get_principal()
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        match &self.user_agent {
            Some(user_agent) => {
                // Direct transfer from user's own agent
                let ledger = sns_ledger::SnsLedger(ledger_id, user_agent);

                let res = ledger
                    .icrc_1_transfer(sns_ledger::TransferArg {
                        from_subaccount: None,
                        to: LedgerAccount {
                            owner: admin_principal,
                            subaccount: None,
                        },
                        amount: amount.into(),
                        fee: None,
                        memo: None,
                        created_at_time: None,
                    })
                    .await
                    .map_err(|e| Error::YralCanister(e.to_string()))?;

                match res {
                    sns_ledger::TransferResult::Ok(_) => Ok(amount),
                    sns_ledger::TransferResult::Err(e) => {
                        Err(Error::YralCanister(format!("Transfer failed: {e:?}")))
                    }
                }
            }
            None => {
                // Use transfer_from with admin agent
                let ledger = sns_ledger::SnsLedger(ledger_id, &self.admin_agent);

                let res = ledger
                    .icrc_2_transfer_from(sns_ledger::TransferFromArgs {
                        spender_subaccount: None,
                        from: LedgerAccount {
                            owner: user_principal,
                            subaccount: None,
                        },
                        to: LedgerAccount {
                            owner: admin_principal,
                            subaccount: None,
                        },
                        amount: amount.into(),
                        fee: None,
                        memo: None,
                        created_at_time: None,
                    })
                    .await
                    .map_err(|e| Error::YralCanister(e.to_string()))?;

                match res {
                    sns_ledger::TransferFromResult::Ok(_) => Ok(amount),
                    sns_ledger::TransferFromResult::Err(e) => {
                        Err(Error::YralCanister(format!("Transfer failed: {e:?}")))
                    }
                }
            }
        }
    }

    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()> {
        let ledger_id = Principal::from_text(DOLR_AI_LEDGER_CANISTER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        // Always use admin agent for adding balance (refunds)
        let ledger = sns_ledger::SnsLedger(ledger_id, &self.admin_agent);

        // Transfer from admin to user
        let res = ledger
            .icrc_1_transfer(sns_ledger::TransferArg {
                memo: Some(vec![0].into()),
                amount: amount.into(),
                fee: None,
                from_subaccount: None,
                to: LedgerAccount {
                    owner: user_principal,
                    subaccount: None,
                },
                created_at_time: None,
            })
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        match res {
            sns_ledger::TransferResult::Ok(_) => Ok(()),
            sns_ledger::TransferResult::Err(e) => {
                Err(Error::YralCanister(format!("Transfer failed: {e:?}")))
            }
        }
    }
}

#[derive(Clone)]
pub struct YralProSubscription {
    admin_agent: ic_agent::Agent,
    user_principal: Principal,
}

impl TokenOperations for YralProSubscription {
    async fn load_balance(&self, _user_principal: Principal) -> Result<TokenBalance> {
        let user_info_service = UserInfoService(USER_INFO_SERVICE_ID, &self.admin_agent);

        let user_profile_info_res = user_info_service
            .get_user_profile_details_v_5(self.user_principal)
            .await?;

        match user_profile_info_res {
            Result5::Ok(user_profile_info) => match user_profile_info.subscription_plan {
                SubscriptionPlan::Pro(yral_pro_subscription) => Ok(TokenBalance::new(
                    Nat::from(yral_pro_subscription.free_video_credits_left),
                    0,
                )),
                SubscriptionPlan::Free => Ok(TokenBalance::new(Nat::from(0u64), 0)),
            },
            Result5::Err(e) => Err(Error::YralCanister(format!(
                "Failed to get user profile info: {e:?}"
            ))),
        }
    }

    async fn deduct_balance(&self, _user_principal: Principal, _amount: u64) -> Result<u64> {
        let user_info_service = UserInfoService(USER_INFO_SERVICE_ID, &self.admin_agent);

        let deduct_res = user_info_service
            .remove_pro_plan_free_video_credits(self.user_principal, 1)
            .await?;

        match deduct_res {
            UserInfoResult::Ok => Ok(1),
            UserInfoResult::Err(e) => Err(Error::YralCanister(format!(
                "Failed to deduct Yral Pro credit: {e:?}"
            ))),
        }
    }

    async fn add_balance(&self, _user_principal: Principal, _amount: u64) -> Result<()> {
        let user_info_service = UserInfoService(USER_INFO_SERVICE_ID, &self.admin_agent);

        let deduct_res = user_info_service
            .add_pro_plan_free_video_credits(self.user_principal, 1)
            .await?;

        match deduct_res {
            UserInfoResult::Ok => Ok(()),
            UserInfoResult::Err(e) => Err(Error::YralCanister(format!(
                "Failed to add Yral Pro credit: {e:?}"
            ))),
        }
    }
}

#[derive(Clone)]
pub struct CkBtcOperations {
    admin_agent: ic_agent::Agent,
}

impl CkBtcOperations {
    pub fn new(admin_agent: ic_agent::Agent) -> Self {
        Self { admin_agent }
    }
}

impl TokenOperations for CkBtcOperations {
    async fn load_balance(&self, user_principal: Principal) -> Result<TokenBalance> {
        let ledger_id = Principal::from_text(crate::consts::CKBTC_LEDGER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        let ledger = sns_ledger::SnsLedger(ledger_id, &self.admin_agent);

        let balance = ledger
            .icrc_1_balance_of(LedgerAccount {
                owner: user_principal,
                subaccount: None,
            })
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        Ok(TokenBalance::new(balance, 8))
    }

    async fn deduct_balance(&self, _user_principal: Principal, _amount: u64) -> Result<u64> {
        Err(Error::YralCanister(
            "ckBTC deduction not supported".to_string(),
        ))
    }

    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()> {
        let ledger_id = Principal::from_text(crate::consts::CKBTC_LEDGER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        let ledger = sns_ledger::SnsLedger(ledger_id, &self.admin_agent);

        // Convert u64 to Nat
        let amount_nat = candid::Nat::from(amount);

        // Transfer from admin to user
        let res = ledger
            .icrc_1_transfer(sns_ledger::TransferArg {
                to: LedgerAccount {
                    owner: user_principal,
                    subaccount: None,
                },
                amount: amount_nat,
                fee: None,
                memo: Some(Vec::from("Tournament reward").into()),
                from_subaccount: None,
                created_at_time: None,
            })
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        match res {
            sns_ledger::TransferResult::Ok(_) => Ok(()),
            sns_ledger::TransferResult::Err(e) => {
                Err(Error::YralCanister(format!("ckBTC transfer failed: {e:?}")))
            }
        }
    }

    async fn add_balance_with_memo(
        &self,
        user_principal: Principal,
        amount: u64,
        memo: Option<Vec<u8>>,
    ) -> Result<()> {
        let ledger_id = Principal::from_text(crate::consts::CKBTC_LEDGER)
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        let ledger = sns_ledger::SnsLedger(ledger_id, &self.admin_agent);

        // Convert u64 to Nat
        let amount_nat = candid::Nat::from(amount);

        // Use provided memo or default to "ckBTC transfer"
        let transfer_memo = memo.unwrap_or_else(|| Vec::from("ckBTC transfer"));

        // Transfer from admin to user
        let res = ledger
            .icrc_1_transfer(sns_ledger::TransferArg {
                to: LedgerAccount {
                    owner: user_principal,
                    subaccount: None,
                },
                amount: amount_nat,
                fee: None,
                memo: Some(transfer_memo.into()),
                from_subaccount: None,
                created_at_time: None,
            })
            .await
            .map_err(|e| Error::YralCanister(e.to_string()))?;

        match res {
            sns_ledger::TransferResult::Ok(_) => Ok(()),
            sns_ledger::TransferResult::Err(e) => {
                Err(Error::YralCanister(format!("ckBTC transfer failed: {e:?}")))
            }
        }
    }
}

#[enum_dispatch::enum_dispatch(TokenOperations)]
#[allow(clippy::large_enum_variant)]
pub enum TokenOperationsProvider {
    Sats(SatsOperations),
    Dolr(DolrOperations),
    CkBtc(CkBtcOperations),
    YralProSubscription(YralProSubscription),
}
