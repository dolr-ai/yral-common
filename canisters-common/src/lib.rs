use std::sync::Arc;

use agent_wrapper::AgentWrapper;
use candid::Principal;
use canisters_client::{
    ic::USER_POST_SERVICE_ID,
    individual_user_template::IndividualUserTemplate,
    local::USER_INFO_SERVICE_ID,
    platform_orchestrator::PlatformOrchestrator,
    post_cache::PostCache,
    rate_limits::RateLimits,
    sns_governance::SnsGovernance,
    sns_index::SnsIndex,
    sns_ledger::SnsLedger,
    sns_root::SnsRoot,
    sns_swap::SnsSwap,
    user_index::UserIndex,
    user_info_service::{Result3, Result_, UserInfoService},
    user_post_service::UserPostService,
};
use consts::{
    canister_ids::{PLATFORM_ORCHESTRATOR_ID, POST_CACHE_ID, RATE_LIMITS_ID},
    METADATA_API_BASE,
};
use ic_agent::{identity::DelegatedIdentity, Identity};
use serde::{Deserialize, Serialize};
use types::delegated_identity::DelegatedIdentityWire;
use utils::profile::ProfileDetails;
use yral_metadata_client::MetadataClient;
use yral_metadata_types::{SetUserMetadataReqMetadata, UserMetadataV2};

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

    pub async fn register_new_user(
        id: Arc<DelegatedIdentity>,
        id_wire: Arc<DelegatedIdentityWire>,
    ) -> Result<Self> {
        let service_canister = Self {
            agent: AgentWrapper::build(|b| b.with_arc_identity(id.clone())),
            id: Some(id),
            id_wire: Some(id_wire.clone()),
            user_canister: USER_INFO_SERVICE_ID,
            metadata_client: MetadataClient::with_base_url(METADATA_API_BASE.clone()),
            expiry: id_wire
                .delegation_chain
                .iter()
                .fold(u64::MAX, |res, next_val| {
                    next_val.delegation.expiration.min(res)
                }),
            profile_details: None,
        };

        let user_info_service = service_canister.user_info_service().await;
        let result = user_info_service.register_new_user().await?;

        if let Result_::Err(e) = result {
            // If user already exists on-chain but metadata is missing, log and continue
            if e.to_lowercase().contains("already exists") {
                log::error!(
                    "[register_new_user] User already exists on-chain but metadata missing. Error: {} for user {}. Proceeding to set metadata.",
                    e,
                    service_canister.user_principal().to_text()
                );
            } else {
                return Err(Error::YralCanister(format!(
                    "Failed to register new user: {e} for user {}",
                    service_canister.user_principal().to_text()
                )));
            }
        }

        service_canister
            .metadata_client
            .set_user_metadata(
                service_canister.identity(),
                SetUserMetadataReqMetadata {
                    user_canister_id: USER_INFO_SERVICE_ID,
                    user_name: "".into(),
                },
            )
            .await?;

        Ok(service_canister)
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

    pub async fn authenticate_with_network(auth: DelegatedIdentityWire) -> Result<Canisters<true>> {
        let id: DelegatedIdentity = auth.clone().try_into()?;
        let expiry = id
            .delegation_chain()
            .iter()
            .fold(u64::MAX, |prev_expiry, del| {
                del.delegation.expiration.min(prev_expiry)
            });
        let id = Arc::new(id);
        let auth = Arc::new(auth);
        let metadata_client = MetadataClient::with_base_url(METADATA_API_BASE.clone());
        let maybe_meta = metadata_client
            .get_user_metadata_v2(id.sender().unwrap().to_text())
            .await?;

        let mut canisters;
        if let Some(user_metadata) = maybe_meta.clone() {
            let user_canister_id = user_metadata.user_canister_id;

            canisters = Canisters {
                agent: AgentWrapper::build(|b| b.with_arc_identity(id.clone())),
                id: Some(id.clone()),
                id_wire: Some(auth.clone()),
                user_canister: user_canister_id,
                metadata_client,
                expiry,
                profile_details: None,
            };
        } else {
            //TODO Register new user
            canisters = Self::register_new_user(id, auth).await?;
        }

        if canisters.user_canister == USER_INFO_SERVICE_ID {
            let service_canister = canisters.user_info_service().await;
            let user_profile_details = service_canister
                .get_profile_details_v_4(canisters.user_principal())
                .await?;

            match user_profile_details {
                Result3::Ok(profile_details) => {
                    canisters.profile_details = Some(ProfileDetails::from_service_canister(
                        canisters.user_principal(),
                        maybe_meta.map(|m| m.user_name),
                        profile_details,
                    ));
                }
                Result3::Err(e) => {
                    return Err(Error::YralCanister(format!(
                        "{e} for principal {}",
                        canisters.user_principal()
                    )));
                }
            }
        } else {
            let profile_details = canisters
                .individual_user(canisters.user_canister)
                .await
                .get_profile_details_v_2()
                .await?;

            canisters.profile_details = Some(ProfileDetails::from_canister(
                canisters.user_canister,
                maybe_meta.map(|m| m.user_name),
                profile_details,
            ));
        }

        //TODO: update last access time

        Ok(canisters)
    }

    pub async fn set_username(&mut self, new_username: String) -> Result<()> {
        self.metadata_client
            .set_user_metadata(
                self.identity(),
                SetUserMetadataReqMetadata {
                    user_canister_id: self.user_canister,
                    user_name: new_username.clone(),
                },
            )
            .await?;
        if let Some(p) = self.profile_details.as_mut() {
            p.username = Some(new_username)
        }

        Ok(())
    }

    pub fn update_profile_details(
        &mut self,
        bio: Option<String>,
        website_url: Option<String>,
        profile_pic: Option<String>,
    ) {
        if let Some(ref mut profile) = self.profile_details {
            profile.bio = bio;
            profile.website_url = website_url;
            if let Some(pic) = profile_pic {
                profile.profile_pic = Some(pic);
            }
        }
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

    pub async fn user_info_service(&self) -> UserInfoService<'_> {
        let agent = self.agent.get_agent().await;
        UserInfoService(USER_INFO_SERVICE_ID, agent)
    }

    pub async fn user_post_service(&self) -> UserPostService<'_> {
        let agent = self.agent.get_agent().await;
        UserPostService(USER_POST_SERVICE_ID, agent)
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

    pub async fn get_user_metadata(
        &self,
        username_or_principal: String,
    ) -> Result<Option<UserMetadataV2>> {
        let meta = self
            .metadata_client
            .get_user_metadata_v2(username_or_principal)
            .await?;
        Ok(meta)
    }

    pub async fn get_individual_canister_v2(
        &self,
        username_or_principal: String,
    ) -> Result<Option<Principal>> {
        let meta = self.get_user_metadata(username_or_principal).await?;

        Ok(meta.map(|m| m.user_canister_id))
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

    pub async fn rate_limits(&self) -> RateLimits<'_> {
        let agent = self.agent.get_agent().await;
        RateLimits(RATE_LIMITS_ID, agent)
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

pub fn yral_auth_login_hint(identity: &impl Identity) -> identity::Result<String> {
    let msg =
        identity::msg_builder::Message::default().method_name("yral_auth_v2_login_hint".into());
    let sig = identity::ic_agent::sign_message(identity, msg)?;

    #[derive(Serialize)]
    struct LoginHint {
        pub user_principal: Principal,
        pub signature: identity::Signature,
    }

    let login_hint = LoginHint {
        user_principal: identity.sender().unwrap(),
        signature: sig,
    };

    Ok(serde_json::to_string(&login_hint).expect("login hint should serialize"))
}
