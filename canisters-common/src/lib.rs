use std::sync::Arc;

use agent_wrapper::AgentWrapper;
use candid::{Decode, Principal};
use canisters_client::{
    individual_user_template::{IndividualUserTemplate, Result15, Result3, UserCanisterDetails},
    platform_orchestrator::PlatformOrchestrator,
    post_cache::PostCache,
    sns_governance::SnsGovernance,
    sns_index::SnsIndex,
    sns_ledger::SnsLedger,
    sns_root::SnsRoot,
    sns_swap::SnsSwap,
    user_index::{Result_, UserIndex},
};
use consts::{
    canister_ids::{PLATFORM_ORCHESTRATOR_ID, POST_CACHE_ID},
    CDAO_SWAP_TIME_SECS, METADATA_API_BASE,
};
use ic_agent::{identity::DelegatedIdentity, Identity};
use serde::{Deserialize, Serialize};
use sns_validation::pbs::sns_pb::SnsInitPayload;
use types::delegated_identity::DelegatedIdentityWire;
use utils::profile::ProfileDetails;
use yral_metadata_client::MetadataClient;
use yral_metadata_types::SetUserMetadataReqMetadata;

pub mod agent_wrapper;
mod consts;
pub mod cursored_data;
mod error;
pub mod utils;

pub use error::*;
pub const CENT_TOKEN_NAME: &str = "CENTS";
pub const SATS_TOKEN_NAME: &str = "Satoshi";
pub const SATS_TOKEN_SYMBOL: &str = "SATS";

#[derive(Clone)]
pub struct Canisters<const AUTH: bool> {
    agent: AgentWrapper,
    id: Option<Arc<DelegatedIdentity>>,
    id_wire: Option<Arc<DelegatedIdentityWire>>,
    metadata_client: MetadataClient<false>,
    user_canister: Principal,
    expiry: u64,
    profile_details: Option<ProfileDetails>,
}

impl Default for Canisters<false> {
    fn default() -> Self {
        Self {
            agent: AgentWrapper::build(|b| b),
            id: None,
            id_wire: None,
            metadata_client: MetadataClient::with_base_url(METADATA_API_BASE.clone()),
            user_canister: Principal::anonymous(),
            expiry: 0,
            profile_details: None,
        }
    }
}

impl Canisters<true> {
    pub fn expiry_ns(&self) -> u64 {
        self.expiry
    }

    pub fn identity(&self) -> &DelegatedIdentity {
        self.id
            .as_ref()
            .expect("Authenticated canisters must have an identity")
    }

    pub fn user_canister(&self) -> Principal {
        self.user_canister
    }

    pub async fn authenticated_user(&self) -> IndividualUserTemplate<'_> {
        self.individual_user(self.user_canister).await
    }

    pub async fn deploy_cdao_sns(&self, init_payload: SnsInitPayload) -> Result<Result3> {
        let agent = self.agent.get_agent().await;
        let args = candid::encode_args((init_payload, CDAO_SWAP_TIME_SECS)).unwrap();
        let bytes = agent
            .update(&self.user_canister, "deploy_cdao_sns")
            .with_arg(args)
            .call_and_wait()
            .await?;
        Ok(Decode!(&bytes, Result3)?)
    }

    pub fn profile_details(&self) -> ProfileDetails {
        self.profile_details
            .clone()
            .expect("Authenticated canisters must have profile details")
    }

    pub fn user_principal(&self) -> Principal {
        self.identity()
            .sender()
            .expect("expect principal to be present")
    }

    async fn create_individual_canister(&self) -> Result<Principal> {
        let subnet_idxs = self.subnet_indexes().await?;

        let mut by = [0u8; 16];
        let principal = self.identity().sender().unwrap();
        let principal_by = principal.as_slice();
        let cnt = by.len().min(principal_by.len());
        by[..cnt].copy_from_slice(&principal_by[..cnt]);

        let discrim = u128::from_be_bytes(by);
        let subnet_idx = subnet_idxs[(discrim % subnet_idxs.len() as u128) as usize];
        let idx = self.user_index_with(subnet_idx).await;
        let user_canister = match idx
            .get_requester_principals_canister_id_create_if_not_exists()
            .await?
        {
            Result_::Ok(val) => Ok(val),
            Result_::Err(e) => Err(Error::YralCanister(e)),
        }?;

        self.metadata_client
            .set_user_metadata(
                self.identity(),
                SetUserMetadataReqMetadata {
                    user_canister_id: user_canister,
                    user_name: "".into(),
                },
            )
            .await?;

        Ok(user_canister)
    }

    async fn handle_referrer(&self, referrer: Principal) -> Result<()> {
        let user = self.authenticated_user().await;

        let maybe_referrer_canister = self
            .get_individual_canister_by_user_principal(referrer)
            .await?;
        let Some(referrer_canister) = maybe_referrer_canister else {
            return Ok(());
        };

        user.update_referrer_details(UserCanisterDetails {
            user_canister_id: referrer_canister,
            profile_owner: referrer,
        })
        .await?;

        Ok(())
    }

    pub async fn authenticate_with_network(
        auth: DelegatedIdentityWire,
        referrer: Option<Principal>,
    ) -> Result<Self> {
        let id: DelegatedIdentity = auth.clone().try_into()?;
        let expiry = id
            .delegation_chain()
            .iter()
            .fold(u64::MAX, |prev_expiry, del| {
                del.delegation.expiration.min(prev_expiry)
            });
        let id = Arc::new(id);
        let mut res = Self {
            agent: AgentWrapper::build(|b| b.with_arc_identity(id.clone())),
            metadata_client: MetadataClient::with_base_url(METADATA_API_BASE.clone()),
            id: Some(id.clone()),
            id_wire: Some(Arc::new(auth)),
            user_canister: Principal::anonymous(),
            expiry,
            profile_details: None,
        };

        let maybe_user_canister = res
            .get_individual_canister_by_user_principal(id.sender().unwrap())
            .await?;
        res.user_canister = if let Some(user_canister) = maybe_user_canister {
            user_canister
        } else {
            res.create_individual_canister().await?
        };

        if let Some(referrer_principal_id) = referrer {
            res.handle_referrer(referrer_principal_id).await?;
        }

        let user = res.authenticated_user().await;
        match user
            .update_last_access_time()
            .await
            .map_err(|e| e.to_string())
        {
            Ok(Result15::Ok(_)) => (),
            Err(e) | Ok(Result15::Err(e)) => log::warn!("Failed to update last access time: {e}"),
        }

        res.profile_details = Some(user.get_profile_details().await?.into());

        Ok(res)
    }

    pub fn from_wire(wire: CanistersAuthWire, base: Canisters<false>) -> Result<Self> {
        let id: DelegatedIdentity = wire.id.clone().try_into()?;
        let arc_id = Arc::new(id);

        let mut agent = base.agent.clone();
        agent.set_arc_id(arc_id.clone());

        Ok(Self {
            agent,
            id: Some(arc_id),
            id_wire: Some(Arc::new(wire.id)),
            metadata_client: base.metadata_client,
            user_canister: wire.user_canister,
            expiry: wire.expiry,
            profile_details: Some(wire.profile_details),
        })
    }
}

impl<const A: bool> Canisters<A> {
    pub async fn post_cache(&self) -> PostCache<'_> {
        let agent = self.agent.get_agent().await;
        PostCache(POST_CACHE_ID, agent)
    }

    pub async fn individual_user(&self, user_canister: Principal) -> IndividualUserTemplate<'_> {
        let agent = self.agent.get_agent().await;
        IndividualUserTemplate(user_canister, agent)
    }

    pub async fn user_index_with(&self, subnet_principal: Principal) -> UserIndex<'_> {
        let agent = self.agent.get_agent().await;
        UserIndex(subnet_principal, agent)
    }

    pub async fn orchestrator(&self) -> PlatformOrchestrator<'_> {
        let agent = self.agent.get_agent().await;
        PlatformOrchestrator(PLATFORM_ORCHESTRATOR_ID, agent)
    }

    pub async fn get_individual_canister_by_user_principal(
        &self,
        user_principal: Principal,
    ) -> Result<Option<Principal>> {
        let meta = self
            .metadata_client
            .get_user_metadata(user_principal)
            .await?;
        if let Some(meta) = meta {
            return Ok(Some(meta.user_canister_id));
        }
        #[cfg(feature = "local")]
        {
            Ok(None)
        }
        #[cfg(not(feature = "local"))]
        {
            Ok(None)
        }
    }

    pub async fn sns_governance(&self, canister_id: Principal) -> SnsGovernance<'_> {
        let agent = self.agent.get_agent().await;
        SnsGovernance(canister_id, agent)
    }

    pub async fn sns_index(&self, canister_id: Principal) -> SnsIndex<'_> {
        let agent = self.agent.get_agent().await;
        SnsIndex(canister_id, agent)
    }

    pub async fn sns_ledger(&self, canister_id: Principal) -> SnsLedger<'_> {
        let agent = self.agent.get_agent().await;
        SnsLedger(canister_id, agent)
    }

    pub async fn sns_root(&self, canister_id: Principal) -> SnsRoot<'_> {
        let agent = self.agent.get_agent().await;
        SnsRoot(canister_id, agent)
    }

    pub async fn sns_swap(&self, canister_id: Principal) -> SnsSwap<'_> {
        let agent = self.agent.get_agent().await;
        SnsSwap(canister_id, agent)
    }

    async fn subnet_indexes(&self) -> Result<Vec<Principal>> {
        #[cfg(feature = "local")]
        {
            use consts::canister_ids::USER_INDEX_ID;
            Ok(vec![USER_INDEX_ID])
        }
        #[cfg(not(feature = "local"))]
        {
            let orchestrator = self.orchestrator().await;
            Ok(orchestrator
                .get_all_available_subnet_orchestrators()
                .await?
                .into_iter()
                .collect())
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CanistersAuthWire {
    pub id: DelegatedIdentityWire,
    pub user_canister: Principal,
    expiry: u64,
    pub profile_details: ProfileDetails,
}

impl From<Canisters<true>> for CanistersAuthWire {
    fn from(value: Canisters<true>) -> Self {
        Self {
            id: value.id_wire.as_ref().unwrap().as_ref().clone(),
            user_canister: value.user_canister(),
            expiry: value.expiry,
            profile_details: value.profile_details(),
        }
    }
}

pub fn yral_auth_login_hint(identity: &impl Identity) -> yral_identity::Result<String> {
    let msg = yral_identity::msg_builder::Message::default()
        .method_name("yral_auth_v2_login_hint".into());
    let sig = yral_identity::ic_agent::sign_message(identity, msg)?;

    #[derive(Serialize)]
    struct LoginHint {
        pub user_principal: Principal,
        pub signature: yral_identity::Signature,
    }

    let login_hint = LoginHint {
        user_principal: identity.sender().unwrap(),
        signature: sig,
    };

    Ok(serde_json::to_string(&login_hint).expect("login hint should serialize"))
}
