use canisters_client::sns_swap::GetInitArg;
use pump_n_dump_common::{rest::BalanceInfoResponse, WithdrawalState};
use std::{fmt::Display, str::FromStr};

use balance::{TokenBalance, TokenBalanceOrClaiming};
use candid::{Nat, Principal};
use grpc_traits::TokenInfoProvider;
use ic_agent::export::PrincipalError;

use crate::{
    consts::{
        CKBTC_INDEX, CKBTC_LEDGER, CKUSDC_INDEX, CKUSDC_LEDGER, PUMP_AND_DUMP_WORKER_URL,
        SUPPORTED_NON_YRAL_TOKENS_ROOT,
    },
    error, Canisters, PndError, Result, CENT_TOKEN_NAME,
};
use canisters_client::{
    sns_governance::{DissolveState, GetMetadataArg, ListNeurons},
    sns_ledger::{self, Account as LedgerAccount, MetadataValue},
    sns_root::ListSnsCanistersArg,
};
use serde::{Deserialize, Serialize};
pub mod balance;

use canisters_client::individual_user_template::ClaimStatus;
use canisters_client::sns_root::ListSnsCanistersResponse;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenMetadata {
    pub logo_b64: String,
    pub name: String,
    pub description: String,
    pub symbol: String,
    pub balance: Option<TokenBalanceOrClaiming>,
    /// applicable for cents only
    pub withdrawable_state: Option<WithdrawalState>,
    pub fees: TokenBalance,
    pub root: Option<Principal>,
    pub ledger: Principal,
    pub index: Principal,
    pub decimals: u8,
    #[serde(default)]
    pub is_nsfw: bool,
    #[serde(default)]
    pub token_owner: Option<TokenOwner>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Hash, Eq, Debug)]
pub enum RootType {
    BTC { ledger: Principal, index: Principal },
    USDC { ledger: Principal, index: Principal },
    COYNS,
    CENTS,
    Other(Principal),
}

impl FromStr for RootType {
    type Err = PrincipalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "btc" => Ok(Self::BTC {
                ledger: Principal::from_text(CKBTC_LEDGER)?,
                index: Principal::from_text(CKBTC_INDEX)?,
            }),
            "usdc" => Ok(Self::USDC {
                ledger: Principal::from_text(CKUSDC_LEDGER)?,
                index: Principal::from_text(CKUSDC_INDEX)?,
            }),
            "coyns" => Ok(Self::COYNS),
            "cents" => Ok(Self::CENTS),
            _ => Ok(Self::Other(Principal::from_text(s)?)),
        }
    }
}

impl Display for RootType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BTC { .. } => f.write_str("btc"),
            Self::USDC { .. } => f.write_str("usdc"),
            Self::COYNS => f.write_str("coyns"),
            Self::CENTS => f.write_str("cents"),
            Self::Other(principal) => f.write_str(&principal.to_text()),
        }
    }
}

async fn load_cents_balance(
    user_canister: Principal,
) -> std::result::Result<BalanceInfoResponse, PndError> {
    let balance_url = PUMP_AND_DUMP_WORKER_URL
        .join(&format!("/balance/{user_canister}"))
        .expect("Url to be valid");

    let res: BalanceInfoResponse = reqwest::get(balance_url).await?.json().await?;

    Ok(res)
}

impl<const A: bool> Canisters<A> {
    pub async fn token_metadata_by_root_type(
        &self,
        nsfw_detector: &impl TokenInfoProvider,
        user_principal: Option<Principal>,
        root_type: RootType,
    ) -> Result<Option<TokenMetadata>> {
        match root_type {
            RootType::BTC { ledger, index } | RootType::USDC { ledger, index } => {
                self.get_ck_metadata(user_principal, ledger, index).await
            }
            RootType::COYNS => {
                let Some(user_principal) = user_principal else {
                    return Ok(None);
                };

                let Some(user_canister) = self
                    .get_individual_canister_by_user_principal(user_principal)
                    .await?
                else {
                    return Ok(None);
                };

                let indiv = self.individual_user(user_canister).await;

                let bal = indiv.get_utility_token_balance().await?;

                Ok(Some(TokenMetadata {
                    logo_b64: "/img/common/coyns.webp".to_string(),
                    name: "COYNS".to_string(),
                    description: "".to_string(),
                    symbol: "COYNS".to_string(),
                    balance: Some(TokenBalanceOrClaiming::new(TokenBalance::new(
                        bal.into(),
                        0,
                    ))),
                    withdrawable_state: None,
                    fees: TokenBalance::new(0u32.into(), 0),
                    root: None,
                    ledger: Principal::anonymous(),
                    index: Principal::anonymous(),
                    decimals: 8,
                    is_nsfw: false,
                    token_owner: None,
                }))
            }
            RootType::CENTS => {
                let Some(user_principal) = user_principal else {
                    return Ok(None);
                };

                let Some(user_canister) = self
                    .get_individual_canister_by_user_principal(user_principal)
                    .await?
                else {
                    return Ok(None);
                };

                let bal_info = load_cents_balance(user_canister).await?;
                let bal = bal_info.balance.clone();

                let withdrawal_state = if bal_info.withdrawable == 0usize {
                    WithdrawalState::NeedMoreEarnings(
                        (bal_info.net_airdrop_reward - bal_info.balance) + 1e6 as usize,
                    )
                } else {
                    WithdrawalState::Value(bal_info.withdrawable)
                };

                Ok(Some(TokenMetadata {
                    logo_b64: "/img/pumpdump/cents.webp".to_string(),
                    name: CENT_TOKEN_NAME.into(),
                    description: "".to_string(),
                    symbol: CENT_TOKEN_NAME.into(),
                    balance: Some(TokenBalanceOrClaiming::new(TokenBalance::new(bal, 6))),
                    withdrawable_state: Some(withdrawal_state),
                    fees: TokenBalance::new(0u32.into(), 0),
                    root: None,
                    ledger: Principal::anonymous(),
                    index: Principal::anonymous(),
                    decimals: 8,
                    is_nsfw: false,
                    token_owner: None,
                }))
            }
            RootType::Other(root) => {
                self.token_metadata_by_root(nsfw_detector, user_principal, root)
                    .await
            }
        }
    }

    pub async fn token_metadata_by_root(
        &self,
        nsfw_detector: &impl TokenInfoProvider,
        user_principal: Option<Principal>,
        token_root: Principal,
    ) -> Result<Option<TokenMetadata>> {
        let root = self.sns_root(token_root).await;
        let sns_cans = root.list_sns_canisters(ListSnsCanistersArg {}).await?;
        let Some(governance) = sns_cans.governance else {
            return Ok(None);
        };
        let Some(ledger) = sns_cans.ledger else {
            return Ok(None);
        };
        let Some(index) = sns_cans.index else {
            return Ok(None);
        };

        let metadata = self
            .get_token_metadata(
                nsfw_detector,
                user_principal,
                token_root,
                governance,
                ledger,
                index,
            )
            .await?;

        Ok(Some(metadata))
    }

    pub async fn get_token_metadata(
        &self,
        nsfw_detector: &impl TokenInfoProvider,
        user_principal: Option<Principal>,
        token_root: Principal,
        governance: Principal,
        ledger: Principal,
        index: Principal,
    ) -> Result<TokenMetadata> {
        let governance_can = self.sns_governance(governance).await;
        let metadata = governance_can.get_metadata(GetMetadataArg {}).await?;

        let ledger_can = self.sns_ledger(ledger).await;
        let symbol = ledger_can.icrc_1_symbol().await?;

        let fees = ledger_can.icrc_1_fee().await?;
        let decimals = ledger_can.icrc_1_decimals().await?;

        let is_nsfw = nsfw_detector
            .get_token_by_id(token_root.to_string())
            .await
            .map(|token_info| token_info.is_nsfw)
            .unwrap_or(false);

        let token_owner = self.get_token_owner(token_root).await?;

        let mut token_metadata = TokenMetadata {
            logo_b64: metadata.logo.unwrap_or_default(),
            name: metadata.name.unwrap_or_default(),
            description: metadata.description.unwrap_or_default(),
            symbol,
            fees: TokenBalance::new_cdao(fees),
            balance: None,
            withdrawable_state: None,
            root: Some(token_root),
            ledger,
            index,
            decimals,
            is_nsfw,
            token_owner,
        };

        if let Some(user_principal) = user_principal {
            let balance = self
                .get_token_balance(user_principal, governance, ledger, token_metadata.decimals)
                .await?;
            token_metadata.balance = Some(balance);
        }

        Ok(token_metadata)
    }

    pub async fn get_token_balance(
        &self,
        user_principal: Principal,
        governance: Principal,
        ledger: Principal,
        decimals: u8,
    ) -> Result<TokenBalanceOrClaiming> {
        let ledger = self.sns_ledger(ledger).await;
        let acc = LedgerAccount {
            owner: user_principal,
            subaccount: None,
        };
        // Balance > 0 -> Token is already claimed
        let balance_e8s = ledger.icrc_1_balance_of(acc).await?;
        let ready_balance = |e8s| {
            Ok(TokenBalanceOrClaiming::new(TokenBalance::new(
                e8s, decimals,
            )))
        };
        if balance_e8s > 0u8 {
            return ready_balance(balance_e8s);
        }

        // if balance is 0 we may not have completed claiming
        let governance = self.sns_governance(governance).await;
        let neurons = governance
            .list_neurons(ListNeurons {
                of_principal: Some(user_principal),
                limit: 10,
                start_page_at: None,
            })
            .await?
            .neurons;

        if neurons.len() < 2 || neurons[1].cached_neuron_stake_e8s == 0 {
            return ready_balance(balance_e8s);
        }

        if matches!(
            neurons[1].dissolve_state.as_ref(),
            Some(DissolveState::DissolveDelaySeconds(0))
        ) {
            return Ok(TokenBalanceOrClaiming::claiming());
        }

        if neurons[0].cached_neuron_stake_e8s == 0 {
            return ready_balance(balance_e8s);
        }

        Ok(TokenBalanceOrClaiming::claiming())
    }

    pub async fn get_ck_metadata(
        &self,
        user_principal: Option<Principal>,
        ledger: Principal,
        index: Principal,
    ) -> Result<Option<TokenMetadata>> {
        let ledger_can = self.sns_ledger(ledger).await;
        let Ok(metadata) = ledger_can.icrc_1_metadata().await else {
            return Ok(None);
        };

        let mut logo_b64 = None::<String>;
        let mut name = None::<String>;
        let mut decimals = None::<Nat>;
        let mut symbol = None::<String>;
        let mut fees = None::<Nat>;

        for (k, v) in metadata {
            if k == "icrc1:logo" {
                let MetadataValue::Text(logo) = v else {
                    return Ok(None);
                };
                logo_b64 = Some(logo);
                continue;
            }
            if k == "icrc1:name" {
                let MetadataValue::Text(name_v) = v else {
                    return Ok(None);
                };
                name = Some(name_v);
                continue;
            }
            if k == "icrc1:decimals" {
                let MetadataValue::Nat(decimals_v) = v else {
                    return Ok(None);
                };
                decimals = Some(decimals_v);
                continue;
            }
            if k == "icrc1:symbol" {
                let MetadataValue::Text(symbol_v) = v else {
                    return Ok(None);
                };
                symbol = Some(symbol_v);
                continue;
            }
            if k == "icrc1:fee" {
                let MetadataValue::Nat(fee_v) = v else {
                    return Ok(None);
                };
                fees = Some(fee_v);
                continue;
            }
        }

        let Some(logo_b64) = logo_b64 else {
            return Ok(None);
        };
        let Some(name) = name else {
            return Ok(None);
        };
        let Some(decimals) = decimals else {
            return Ok(None);
        };
        let Some(symbol) = symbol else {
            return Ok(None);
        };
        let Some(fees) = fees else {
            return Ok(None);
        };

        let decimals: u8 = decimals.0.try_into().unwrap();
        let mut res = TokenMetadata {
            logo_b64,
            name: name.replace("ck", ""),
            description: String::new(),
            symbol: symbol.replace("ck", ""),
            balance: None,
            withdrawable_state: None,
            fees: TokenBalance::new(fees, decimals),
            root: None,
            ledger,
            index,
            decimals,
            is_nsfw: false,
            token_owner: None,
        };
        let Some(user_principal) = user_principal else {
            return Ok(Some(res));
        };

        let Ok(bal) = ledger_can
            .icrc_1_balance_of(LedgerAccount {
                owner: user_principal,
                subaccount: None,
            })
            .await
        else {
            return Ok(None);
        };
        res.balance = Some(TokenBalanceOrClaiming::new(TokenBalance::new(
            bal, decimals,
        )));

        Ok(Some(res))
    }

    pub async fn transfer_token_to_user_principal(
        &self,
        destination: Principal,
        ledger_id: Principal,
        root_id: Principal,
        amount: TokenBalance,
    ) -> Result<()> {
        let sns_ledger = self.sns_ledger(ledger_id).await;
        let res = sns_ledger
            .icrc_1_transfer(sns_ledger::TransferArg {
                memo: Some(vec![0].into()),
                amount: amount.clone().into(),
                fee: None,
                from_subaccount: None,
                to: LedgerAccount {
                    owner: destination,
                    subaccount: None,
                },
                created_at_time: None,
            })
            .await?;
        log::debug!("transfer res: {:?}", res);

        let destination_canister_id = self
            .get_individual_canister_by_user_principal(destination)
            .await?;
        let Some(destination_canister_id) = destination_canister_id else {
            return Ok(());
        };
        let is_non_yral_token = SUPPORTED_NON_YRAL_TOKENS_ROOT
            .iter()
            .any(|&token_root| token_root == root_id.to_text());
        if is_non_yral_token {
            return Ok(());
        }

        let destination_canister = self.individual_user(destination_canister_id).await;
        let res = destination_canister.add_token(root_id).await?;
        log::debug!("add_token res {res:?}");

        Ok(())
    }

    pub async fn transfer_ck_token_to_user_principal(
        &self,
        destination: Principal,
        ledger_id: Principal,
        amount: TokenBalance,
    ) -> Result<()> {
        let sns_ledger = self.sns_ledger(ledger_id).await;
        let res = sns_ledger
            .icrc_1_transfer(sns_ledger::TransferArg {
                memo: Some(vec![0].into()),
                amount: amount.clone().into(),
                fee: None,
                from_subaccount: None,
                to: LedgerAccount {
                    owner: destination,
                    subaccount: None,
                },
                created_at_time: None,
            })
            .await?;
        log::debug!("transfer res: {:?}", res);
        Ok(())
    }

    pub async fn get_airdrop_status(
        &self,
        token_owner: Principal,
        token_root: Principal,
        user_principal: Principal,
    ) -> Result<bool> {
        let token_owner = self.individual_user(token_owner).await;
        let is_airdrop_claimed = token_owner
            .deployed_cdao_canisters()
            .await?
            .into_iter()
            .any(|token| {
                token.root == token_root
                    && token
                        .airdrop_info
                        .principals_who_successfully_claimed
                        .iter()
                        .any(|(principal, status)| {
                            principal == &user_principal && *status == ClaimStatus::Claimed
                        })
            });
        Ok(is_airdrop_claimed)
    }

    pub async fn get_token_owner(&self, token_root: Principal) -> Result<Option<TokenOwner>> {
        let root = self.sns_root(token_root).await;
        let ListSnsCanistersResponse { swap, .. } =
            root.list_sns_canisters(ListSnsCanistersArg {}).await?;

        let swap = self.sns_swap(swap.unwrap()).await;

        let init = swap.get_init(GetInitArg {}).await?;

        let token_owner_details = init
            .init
            .unwrap()
            .fallback_controller_principal_ids
            .into_iter()
            .collect::<Vec<_>>();

        let canister_id: Option<Principal> = token_owner_details
            .iter()
            .find(|controller| controller.ends_with("-cai"))
            .map(Principal::from_text)
            .transpose()
            .map_err(|e| error::Error::YralCanister(e.to_string()))?;
        let principal_id: Option<Principal> = token_owner_details
            .iter()
            .find(|controller| !controller.ends_with("-cai"))
            .map(Principal::from_text)
            .transpose()
            .map_err(|e| error::Error::YralCanister(e.to_string()))?;
        match (canister_id, principal_id) {
            (Some(canister_id), Some(principal_id)) => Ok(Some(TokenOwner {
                principal_id,
                canister_id,
            })),
            _ => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenOwner {
    pub principal_id: Principal,
    pub canister_id: Principal,
}
