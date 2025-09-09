use candid::Principal;
use hon_worker_common::SatsBalanceUpdateRequestV2;
use num_bigint::{BigInt, BigUint, Sign};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use url::Url;

use super::balance::TokenBalance;
use super::operations::TokenOperations;
use crate::{consts::DOLR_AI_LEDGER_CANISTER, error::Error, Result};
use canisters_client::sns_ledger::{self, Account as LedgerAccount};

// ckBTC transfer types (duplicated from worker for now)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CkBtcTransferRequest {
    pub amount: u64, // Amount in satoshis
    pub reason: Option<String>,
    pub metadata: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CkBtcTransferResponse {
    pub success: bool,
    pub amount: u64,
    pub recipient: String,
}

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
pub struct CkBtcOperations {
    jwt_token: Option<String>,
    client: Client,
}

impl CkBtcOperations {
    pub fn new(jwt_token: Option<String>) -> Self {
        Self {
            jwt_token,
            client: Client::new(),
        }
    }
}

impl TokenOperations for CkBtcOperations {
    async fn load_balance(&self, _user_principal: Principal) -> Result<TokenBalance> {
        // ckBTC balance checking not needed for rewards, return 0
        Ok(TokenBalance::new(0u64.into(), 8))
    }
    
    async fn deduct_balance(&self, _user_principal: Principal, _amount: u64) -> Result<u64> {
        Err(Error::YralCanister("ckBTC deduction not supported".to_string()))
    }
    
    async fn add_balance(&self, user_principal: Principal, amount: u64) -> Result<()> {
        let jwt_token = self.jwt_token.as_ref().ok_or_else(|| {
            Error::YralCanister("JWT token required for ckBTC transfer".to_string())
        })?;
        
        let url: Url = hon_worker_common::WORKER_URL.parse().unwrap();
        let transfer_url = url
            .join(&format!("/v2/transfer_ckbtc/{user_principal}"))
            .expect("Url to be valid");
        
        let request = CkBtcTransferRequest {
            amount,
            reason: Some("tournament_reward".to_string()),
            metadata: None,
        };
        
        let res = self
            .client
            .post(transfer_url)
            .bearer_auth(jwt_token)
            .json(&request)
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
                "Failed to transfer ckBTC: {error_text}"
            )))
        }
    }
}

#[enum_dispatch::enum_dispatch(TokenOperations)]
#[allow(clippy::large_enum_variant)]
pub enum TokenOperationsProvider {
    Sats(SatsOperations),
    Dolr(DolrOperations),
    CkBtc(CkBtcOperations),
}
