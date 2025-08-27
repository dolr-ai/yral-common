use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use candid::Principal;
use canisters_client::individual_user_template::PostDetailsForFrontend;
use canisters_client::{
    ic::USER_INFO_SERVICE_ID,
    user_post_service::{
        Post as PostFromServiceCanister,
        PostDetailsForFrontend as PostServicePostDetailsForFrontend, Result2, Result4,
    },
};
use global_constants::{NSFW_THRESHOLD, USERNAME_MAX_LEN};
use serde::{Deserialize, Serialize};
use username_gen::random_username_from_principal;
use web_time::Duration;

use crate::{Canisters, Result};

use super::profile::propic_from_principal;

#[derive(Debug, Deserialize)]
struct NsfwApiResponse {
    nsfw_probability: f32,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PostDetails {
    pub canister_id: Principal, // canister id of the publishing canister.
    pub post_id: String,
    pub uid: String,
    pub description: String,
    pub views: u64,
    pub likes: u64,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub propic_url: String,
    /// Whether post is liked by the authenticated
    /// user or not, None if unknown
    pub liked_by_user: Option<bool>,
    pub poster_principal: Principal,
    pub hastags: Vec<String>,
    pub is_nsfw: bool,
    pub hot_or_not_feed_ranking_score: Option<u64>,
    pub created_at: Duration,
    pub nsfw_probability: f32,
}

impl PartialOrd for PostDetails {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PostDetails {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created_at.cmp(&other.created_at)
    }
}

impl Hash for PostDetails {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canister_id.hash(state);
        self.post_id.hash(state);
    }
}

impl Eq for PostDetails {}

impl PostDetails {
    pub fn from_canister_post(
        authenticated: bool,
        canister_id: Principal,
        details: PostDetailsForFrontend,
    ) -> Self {
        Self::from_canister_post_with_nsfw_info(authenticated, canister_id, details, 0.0)
    }

    pub fn from_service_post_anonymous(
        canister_id: Principal,
        service_post: PostFromServiceCanister,
    ) -> Self {
        Self {
            canister_id,
            post_id: service_post.id,
            uid: service_post.video_uid,
            description: service_post.description,
            views: service_post.view_stats.total_view_count,
            likes: service_post.likes.len() as u64,
            display_name: None,
            propic_url: propic_from_principal(service_post.creator_principal),
            liked_by_user: None,
            poster_principal: service_post.creator_principal,
            hastags: service_post.hashtags,
            is_nsfw: false,
            hot_or_not_feed_ranking_score: Some(0),
            created_at: Duration::new(
                service_post.created_at.secs_since_epoch,
                service_post.created_at.nanos_since_epoch,
            ),
            nsfw_probability: 0.0,
            username: None,
        }
    }

    pub fn from_service_post(
        canister_id: Principal,
        post_details: PostServicePostDetailsForFrontend,
    ) -> Self {
        Self {
            canister_id,
            post_id: post_details.id,
            uid: post_details.video_uid,
            description: post_details.description,
            views: post_details.total_view_count,
            likes: post_details.like_count,
            display_name: None,
            propic_url: propic_from_principal(post_details.created_by_user_principal_id),
            liked_by_user: Some(post_details.liked_by_me),
            poster_principal: post_details.creator_principal,
            hastags: post_details.hashtags,
            is_nsfw: false,
            hot_or_not_feed_ranking_score: Some(0),
            created_at: Duration::new(
                post_details.created_at.secs_since_epoch,
                post_details.created_at.nanos_since_epoch,
            ),
            nsfw_probability: 0.0,
            username: None,
        }
    }

    pub fn from_canister_post_with_nsfw_info(
        authenticated: bool,
        canister_id: Principal,
        details: PostDetailsForFrontend,
        nsfw_probability: f32,
    ) -> Self {
        Self {
            canister_id,
            post_id: details.id.to_string(),
            uid: details.video_uid,
            description: details.description,
            views: details.total_view_count,
            likes: details.like_count,
            display_name: details.created_by_display_name,
            username: details.created_by_unique_user_name,
            propic_url: details
                .created_by_profile_photo_url
                .unwrap_or_else(|| propic_from_principal(details.created_by_user_principal_id)),
            liked_by_user: authenticated.then_some(details.liked_by_me),
            poster_principal: details.created_by_user_principal_id,
            hastags: details.hashtags,
            is_nsfw: nsfw_probability >= NSFW_THRESHOLD,
            hot_or_not_feed_ranking_score: details.hot_or_not_feed_ranking_score,
            created_at: Duration::new(
                details.created_at.secs_since_epoch,
                details.created_at.nanos_since_epoch,
            ),
            nsfw_probability,
        }
    }

    pub fn is_hot_or_not(&self) -> bool {
        self.hot_or_not_feed_ranking_score.is_some()
    }

    pub fn username_or_principal(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.poster_principal.to_text())
    }

    /// Get the user's username
    /// or a consistent random username
    /// WARN: do not use this method for URLs
    /// use `username_or_principal` instead
    pub fn username_or_fallback(&self) -> String {
        self.username.clone().unwrap_or_else(|| {
            random_username_from_principal(self.poster_principal, USERNAME_MAX_LEN)
        })
    }

    pub fn display_name_or_fallback(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username_or_fallback())
    }
}

impl<const A: bool> Canisters<A> {
    async fn fetch_nsfw_probability(&self, video_uid: &str) -> Result<f32> {
        let url = format!("https://icp-off-chain-agent.fly.dev/api/v2/posts/nsfw_prob/{video_uid}");

        let response = reqwest::get(&url).await?;

        let nsfw_response: NsfwApiResponse = response.json().await?;

        Ok(nsfw_response.nsfw_probability)
    }

    pub async fn get_post_details(
        &self,
        user_canister: Principal,
        post_id: String,
    ) -> Result<Option<PostDetails>> {
        self.get_post_details_with_nsfw_info(user_canister, post_id, None)
            .await
    }

    pub async fn get_post_details_with_nsfw_info(
        &self,
        user_canister: Principal,
        post_id: String,
        nsfw_probability: Option<f32>,
    ) -> Result<Option<PostDetails>> {
        let profile_details = if user_canister == USER_INFO_SERVICE_ID {
            let post_service_canister = self.user_post_service().await;
            let post_details_res = post_service_canister
                .get_individual_post_details_by_id_for_user(
                    post_id.clone(),
                    post_service_canister.1.get_principal().unwrap(),
                )
                .await;

            match post_details_res {
                Ok(post_details) => {
                    let Result2::Ok(post_details) = post_details else {
                        return Ok(None);
                    };
                    Some(PostDetails::from_service_post(user_canister, post_details))
                }
                Err(e) => {
                    log::warn!(
                        "Failed to get post details for {user_canister} {post_id}: {e}, skipping"
                    );
                    None
                }
            }
        } else {
            let post_creator_can = self.individual_user(user_canister).await;
            match post_creator_can
                .get_individual_post_details_by_id(post_id.parse::<u64>().unwrap())
                .await
            {
                Ok(p) => Some(PostDetails::from_canister_post_with_nsfw_info(
                    A,
                    user_canister,
                    p,
                    0.0,
                )),
                Err(e) => {
                    log::warn!(
                        "failed to get post details for {user_canister} {post_id}: {e}, skipping"
                    );
                    None
                }
            }
        };

        let Some(mut post_details) = profile_details else {
            return Ok(None);
        };

        let creator_principal = post_details.poster_principal;
        let creator_meta = self
            .metadata_client
            .get_user_metadata_v2(creator_principal.to_text())
            .await?;
        post_details.username = creator_meta.map(|m| m.user_name).filter(|s| !s.is_empty());

        // Determine NSFW probability: use provided value, or fetch from API, or default to 1.0
        let nsfw_prob = nsfw_probability.unwrap_or(
            self.fetch_nsfw_probability(&post_details.uid)
                .await
                .inspect_err(|e| {
                    log::warn!(
                        "Failed to fetch NSFW probability for video {}: {}, defaulting to 1.0",
                        post_details.uid,
                        e
                    );
                })
                .unwrap_or(1.0),
        );

        post_details.nsfw_probability = nsfw_prob;

        Ok(Some(post_details))
    }

    pub async fn post_like_info(
        &self,
        post_canister: Principal,
        post_id: String,
    ) -> Result<(bool, u64)> {
        let post_details = self.get_post_details(post_canister, post_id).await?;
        let Some(post_details) = post_details else {
            return Err(crate::Error::YralCanister("Post not found".to_string()));
        };

        Ok((
            post_details.liked_by_user.unwrap_or(false),
            post_details.likes,
        ))
    }
}

impl Canisters<true> {
    pub async fn like_post(&self, post_canister: Principal, post_id: String) -> Result<bool> {
        match post_canister {
            USER_INFO_SERVICE_ID => {
                let post_service_canister = self.user_post_service().await;
                let res = post_service_canister
                    .update_post_toggle_like_status_by_caller(post_id)
                    .await?;
                match res {
                    Result4::Ok(val) => Ok(val),
                    Result4::Err(err) => Err(crate::Error::YralCanister(format!("{err:?}"))),
                }
            }
            _ => {
                let individual = self.individual_user(post_canister).await;
                individual
                    .update_post_toggle_like_status_by_caller(post_id.parse::<u64>().unwrap())
                    .await
                    .map_err(|e| crate::Error::YralCanister(e.to_string()))
            }
        }
    }
}
