use std::str::FromStr;

use candid::Principal;
use canisters_client::individual_user_template::Result8;
use futures_util::{
    future,
    stream::{self, FuturesOrdered, FuturesUnordered},
    StreamExt,
};
use grpc_traits::TokenInfoProvider;

use crate::{
    consts::SUPPORTED_NON_YRAL_TOKENS_ROOT,
    utils::token::{
        balance::{TokenBalance, TokenBalanceOrClaiming},
        RootType, TokenMetadata,
    },
    Canisters, Error, Result,
};

use super::{CursoredDataProvider, KeyedData, PageEntry};

impl KeyedData for RootType {
    type Key = RootType;

    fn key(&self) -> Self::Key {
        self.clone()
    }
}

#[derive(Clone)]
pub struct TokenRootList<TkInfo: TokenInfoProvider> {
    pub viewer_principal: Principal,
    pub canisters: Canisters<false>,
    pub user_canister: Principal,
    pub user_principal: Principal,
    pub nsfw_detector: TkInfo,
    pub exclude: Vec<RootType>,
}

pub async fn eligible_non_yral_supported_tokens(
    cans: &Canisters<false>,
    nsfw_detector: &impl TokenInfoProvider,
    user_principal: Principal,
) -> Result<Vec<TokenListResponse>> {
    let tasks = SUPPORTED_NON_YRAL_TOKENS_ROOT
        .iter()
        .map(|&token_root| async move {
            let token_root = Principal::from_text(token_root).ok()?;
            let metadata_res = cans
                .token_metadata_by_root(nsfw_detector, Some(user_principal), token_root)
                .await
                .ok()?;
            Some((token_root, metadata_res))
        })
        .collect::<FuturesOrdered<_>>()
        .filter_map(|res| {
            let Some((token_root, Some(metadata))) = res else {
                return future::ready(None);
            };

            if let Some(balance) = &metadata.balance {
                if balance
                    .map_balance_ref(|b| b.e8s > 0u64)
                    .unwrap_or_default()
                {
                    return future::ready(Some(TokenListResponse {
                        root: RootType::Other(token_root),
                        airdrop_claimed: true,
                        token_metadata: metadata,
                    }));
                }
            } else {
                return future::ready(None);
            }

            future::ready(None)
        });

    Ok(tasks.collect().await)
}

#[derive(Clone)]
pub struct TokenListResponse {
    pub root: RootType,
    pub airdrop_claimed: bool,
    pub token_metadata: TokenMetadata,
}

impl KeyedData for TokenListResponse {
    type Key = RootType;

    fn key(&self) -> Self::Key {
        self.root.clone()
    }
}

impl<TkInfo: TokenInfoProvider + Send + Sync> CursoredDataProvider for TokenRootList<TkInfo> {
    type Data = TokenListResponse;
    type Error = Error;

    async fn get_by_cursor_inner(&self, start: usize, end: usize) -> Result<PageEntry<Self::Data>> {
        let user = self.canisters.individual_user(self.user_canister).await;
        let tokens = user
            .get_token_roots_of_this_user_with_pagination_cursor(start as u64, end as u64)
            .await?;

        let mut tokens_fetched = 0;
        let mut tokens: Vec<TokenListResponse> = match tokens {
            Result8::Ok(v) => {
                tokens_fetched = v.len();
                v.into_iter()
                    .map(|t| async move {
                        let root = RootType::from_str(&t.to_text()).unwrap();

                        let metadata = self
                            .canisters
                            .token_metadata_by_root_type(
                                &self.nsfw_detector,
                                Some(self.user_principal),
                                root.clone(),
                            )
                            .await
                            .ok()??;

                        let airdrop_claimed = self
                            .canisters
                            .get_airdrop_status(
                                metadata.token_owner.clone().unwrap().canister_id,
                                Principal::from_text(root.to_string()).unwrap(),
                                self.viewer_principal,
                            )
                            .await
                            .ok()?;

                        Some(TokenListResponse {
                            root,
                            airdrop_claimed,
                            token_metadata: metadata,
                        })
                    })
                    .collect::<FuturesUnordered<_>>()
                    .filter_map(|x| async { x })
                    .collect::<Vec<TokenListResponse>>()
                    .await
            }
            Result8::Err(_) => vec![],
        };

        println!("{tokens_fetched}, {}, {}", end - start, tokens.len());
        let list_end = tokens_fetched < end - start;

        if start == 0 {
            let mut rep = stream::iter(
                [
                    RootType::SATS,
                    RootType::CENTS,
                    RootType::from_str("btc").unwrap(),
                    RootType::from_str("usdc").unwrap(),
                ]
                .into_iter(),
            )
            .filter_map(|root_type| async move {
                match root_type {
                    RootType::BTC { ledger, index } | RootType::USDC { ledger, index } => {
                        let metadata = self
                            .canisters
                            .get_ck_metadata(Some(self.user_principal), ledger, index)
                            .await
                            .inspect_err(|err| {
                                log::error!("error when fetching ck metadata: {err:?}");
                            })
                            .ok()??;

                        log::info!("fetched ck token data for {}", metadata.name);
                        if metadata.balance
                            != Some(TokenBalanceOrClaiming::new(TokenBalance::new_cdao(
                                0u8.into(),
                            )))
                        {
                            Some(TokenListResponse {
                                root: root_type,
                                airdrop_claimed: true,
                                token_metadata: metadata,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .await;

            rep.extend(
                eligible_non_yral_supported_tokens(
                    &self.canisters,
                    &self.nsfw_detector,
                    self.user_principal,
                )
                .await?,
            );
            rep.retain(|item| !self.exclude.contains(&item.root));
            tokens.splice(0..0, rep);
        }
        Ok(PageEntry {
            data: tokens,
            end: list_end,
        })
    }
}
