use candid::Principal;
use canisters_client::{
    individual_user_template::UserProfileDetailsForFrontendV2,
    user_info_service::UserProfileDetailsForFrontendV3,
};
use global_constants::USERNAME_MAX_LEN;
use serde::{Deserialize, Serialize};
use username_gen::random_username_from_principal;

use crate::{
    consts::{GOBGOB_PROPIC_URL, GOBGOB_TOTAL_COUNT},
    Canisters, Result,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileDetails {
    pub username: Option<String>,
    pub lifetime_earnings: u64,
    pub followers_cnt: u64,
    pub following_cnt: u64,
    pub profile_pic: Option<String>,
    pub display_name: Option<String>,
    pub principal: Principal,
    pub user_canister: Principal,
    pub hots: u64,
    pub nots: u64,
}

impl ProfileDetails {
    pub fn from_canister(
        user_canister: Principal,
        username: Option<String>,
        user: UserProfileDetailsForFrontendV2,
    ) -> Self {
        Self {
            username: username.filter(|u| !u.is_empty()),
            lifetime_earnings: user.lifetime_earnings,
            followers_cnt: user.followers_count,
            following_cnt: user.following_count,
            profile_pic: user.profile_picture_url,
            display_name: user.display_name,
            principal: user.principal_id,
            user_canister,
            hots: user.profile_stats.hot_bets_received,
            nots: user.profile_stats.not_bets_received,
        }
    }

    pub fn from_service_canister(
        user_principal: Principal,
        username: Option<String>,
        profile_details: UserProfileDetailsForFrontendV3,
    ) -> Self {
        Self {
            username: username.clone().filter(|u| !u.is_empty()),
            lifetime_earnings: 0,
            followers_cnt: 0,
            following_cnt: 0,
            profile_pic: profile_details.profile_picture_url,
            display_name: username,
            principal: user_principal,
            user_canister: user_principal,
            hots: 0,
            nots: 0,
        }
    }
}

fn index_from_principal(principal: Principal) -> u32 {
    let hash_value = crc32fast::hash(principal.as_slice());
    (hash_value % GOBGOB_TOTAL_COUNT) + 1
}

pub fn propic_from_principal(principal: Principal) -> String {
    let index = index_from_principal(principal);
    format!("{GOBGOB_PROPIC_URL}{index}/public")
}

impl ProfileDetails {
    pub fn username_or_principal(&self) -> String {
        self.username.clone().unwrap_or_else(|| self.principal())
    }

    /// Get the user's username
    /// or a consistent random username
    /// WARN: do not use this method for URLs
    /// use `username_or_principal` instead
    pub fn username_or_fallback(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| random_username_from_principal(self.principal, USERNAME_MAX_LEN))
    }

    pub fn principal(&self) -> String {
        self.principal.to_text()
    }

    pub fn display_name_or_fallback(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username_or_fallback())
    }

    pub fn profile_pic_or_random(&self) -> String {
        let propic = self.profile_pic.clone().unwrap_or_default();
        if !propic.is_empty() {
            return propic;
        }

        propic_from_principal(self.principal)
    }
}

impl<const A: bool> Canisters<A> {
    pub async fn get_profile_details(
        &self,
        username_or_principal: String,
    ) -> Result<Option<ProfileDetails>> {
        let Some(meta) = self
            .metadata_client
            .get_user_metadata_v2(username_or_principal)
            .await?
        else {
            return Ok(None);
        };
        let user_profile = self.individual_user(meta.user_canister_id).await;
        let user = user_profile.get_profile_details_v_2().await?;

        Ok(Some(ProfileDetails::from_canister(
            meta.user_canister_id,
            Some(meta.user_name),
            user,
        )))
    }
}
