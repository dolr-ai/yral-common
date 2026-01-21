use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use candid::Principal;
use canisters_client::individual_user_template::{
    PostDetailsForFrontend, PostStatus as IndividualUserPostStatus,
};
use canisters_client::{
    ic::USER_INFO_SERVICE_ID,
    user_info_service::Result3,
    user_post_service::{
        Post as PostFromServiceCanister,
        PostDetailsForFrontend as PostServicePostDetailsForFrontend,
        PostStatus as ServicePostStatus, Result2 as PostServiceResult2,
        Result3 as PostServiceResult3, Result5,
    },
};
use futures_util::try_join;
use global_constants::{NSFW_THRESHOLD, USERNAME_MAX_LEN};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use username_gen::random_username_from_principal;
use web_time::Duration;

use crate::{Canisters, Error, Result};

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
    pub creator_follows_user: Option<bool>,
    pub user_follows_creator: Option<bool>,
    pub creator_bio: Option<String>,
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
        username: Option<String>,
        canister_id: Principal,
        details: PostDetailsForFrontend,
    ) -> Self {
        Self::from_canister_post_with_nsfw_info(authenticated, username, canister_id, details, 0.0)
    }

    pub fn from_service_post_anonymous(
        username: Option<String>,
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
            creator_follows_user: None,
            user_follows_creator: None,
            creator_bio: None,
            hastags: service_post.hashtags,
            is_nsfw: false,
            hot_or_not_feed_ranking_score: Some(0),
            created_at: Duration::new(
                service_post.created_at.secs_since_epoch,
                service_post.created_at.nanos_since_epoch,
            ),
            nsfw_probability: 0.0,
            username,
        }
    }

    pub fn from_service_post(
        username: Option<String>,
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
            creator_follows_user: None,
            user_follows_creator: None,
            creator_bio: None,
            hastags: post_details.hashtags,
            is_nsfw: false,
            hot_or_not_feed_ranking_score: Some(0),
            created_at: Duration::new(
                post_details.created_at.secs_since_epoch,
                post_details.created_at.nanos_since_epoch,
            ),
            nsfw_probability: 0.0,
            username,
        }
    }

    pub fn from_canister_post_with_nsfw_info(
        authenticated: bool,
        username: Option<String>,
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
            username,
            propic_url: details
                .created_by_profile_photo_url
                .unwrap_or_else(|| propic_from_principal(details.created_by_user_principal_id)),
            liked_by_user: authenticated.then_some(details.liked_by_me),
            poster_principal: details.created_by_user_principal_id,
            creator_follows_user: None,
            user_follows_creator: None,
            creator_bio: None,
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
    #[instrument(skip(self))]
    async fn fetch_nsfw_probability(&self, video_uid: &str) -> Result<f32> {
        let url = format!("https://offchain.yral.com/api/v2/posts/nsfw_prob/{video_uid}");

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

    #[instrument(skip(self))]
    async fn get_individual_post_details_by_id_instrumented(
        &self,
        user_canister: Principal,
        post_id: u64,
    ) -> Option<PostDetailsForFrontend> {
        let post_creator_can = self.individual_user(user_canister).await;
        post_creator_can
            .get_individual_post_details_by_id(post_id)
            .await
            .inspect_err(|err| {
                log::warn!(
                    "failed to get post details for {user_canister} {post_id}: {err:#?}, skipping"
                );
            })
            .ok()
    }

    /// A fast path for fetching post details from the canister.
    ///
    /// No additional detail is resolved, e.g. username or nsfw probability. For
    /// a more accurate post detail refer to `[Canisters::get_post_details]`
    ///
    /// Note: This function filters posts by status, returning `None` for posts
    /// that are banned, deleted, or not yet ready to view.
    #[tracing::instrument(skip(self))]
    pub async fn get_post_details_from_canister(
        &self,
        user_canister: Principal,
        post_id: &str,
    ) -> Result<Option<PostDetails>> {
        let post_details = if user_canister == USER_INFO_SERVICE_ID {
            let post_service_canister = self.user_post_service().await;

            // First, get the post with status to check if it's viewable
            let post_with_status = post_service_canister
                .get_individual_post_details_by_id(post_id.into())
                .await?;

            let PostServiceResult2::Ok(post) = post_with_status else {
                return Ok(None);
            };

            // Check if post status allows viewing
            if !matches!(post.status, ServicePostStatus::ReadyToView) {
                return Ok(None);
            }

            // Get full post details with liked_by_me info
            let post_details = post_service_canister
                .get_individual_post_details_by_id_for_user(
                    post_id.into(),
                    post_service_canister.1.get_principal().unwrap(),
                )
                .await?;

            let PostServiceResult3::Ok(post_details) = post_details else {
                return Ok(None);
            };

            Ok::<_, Error>(Some(PostDetails::from_service_post(
                None,
                user_canister,
                post_details,
            )))
        } else {
            let post_creator_can = self.individual_user(user_canister).await;
            let res = post_creator_can
                .get_individual_post_details_by_id(post_id.parse::<u64>().unwrap())
                .await?;

            // Check if post status allows viewing
            if !matches!(res.status, IndividualUserPostStatus::ReadyToView) {
                return Ok(None);
            }

            Ok(Some(PostDetails::from_canister_post_with_nsfw_info(
                A,
                None,
                user_canister,
                res,
                0.0,
            )))
        }?;

        Ok(post_details)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_post_details_with_nsfw_info(
        &self,
        user_canister: Principal,
        post_id: String,
        nsfw_probability: Option<f32>,
    ) -> Result<Option<PostDetails>> {
        let post_details = self
            .get_post_details_from_canister(user_canister, &post_id)
            .await?;

        let Some(mut post_details) = post_details else {
            return Ok(None);
        };

        let creator_principal = post_details.poster_principal;
        let (creator_meta, nsfw_prob) = try_join!(
            async {
                let meta = self
                    .metadata_client
                    .get_user_metadata_v2(creator_principal.to_text())
                    .await?;

                Ok::<_, Error>(meta)
            },
            async {
                // Determine NSFW probability: use provided value, or fetch from API, or default to 1.0
                if let Some(nsfw_prob) = nsfw_probability {
                    return Ok(nsfw_prob);
                }
                // TODO: add a fast path for fetching nsfw probability
                // since the probablity wont ever change for any given video_uid, it can be easily cached
                Ok(self
                    .fetch_nsfw_probability(&post_details.uid)
                    .await
                    .inspect_err(|e| {
                        log::warn!(
                            "Failed to fetch NSFW probability for video {}: {}, defaulting to 1.0",
                            post_details.uid,
                            e
                        );
                    })
                    .unwrap_or(1.0))
            }
        )?;

        post_details.nsfw_probability = nsfw_prob;
        post_details.username = creator_meta.map(|m| m.user_name).filter(|s| !s.is_empty());

        Ok(Some(post_details))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_post_details_with_creator_info(
        &self,
        user_canister: Principal,
        post_id: String,
    ) -> Result<Option<PostDetails>> {
        // First, get the base post details with NSFW info
        let Some(mut post_details) = self
            .get_post_details_with_nsfw_info(user_canister, post_id, None)
            .await?
        else {
            return Ok(None);
        };

        let creator_principal = post_details.poster_principal;

        // Fetch metadata and profile details in parallel
        let (creator_meta, profile_result) = try_join!(
            async {
                let meta = self
                    .metadata_client
                    .get_user_metadata_v2(creator_principal.to_text())
                    .await?;

                Ok::<_, Error>(meta)
            },
            async {
                let service_canister = self.user_info_service().await;
                let profile_details = service_canister
                    .get_profile_details_v_4(creator_principal)
                    .await?;
                Ok::<_, Error>(Some(profile_details))
            }
        )?;

        // Handle username: use metadata if available, otherwise generate
        post_details.username = if let Some(meta) = creator_meta {
            let username = meta.user_name;
            if !username.is_empty() {
                Some(username)
            } else {
                Some(random_username_from_principal(
                    creator_principal,
                    USERNAME_MAX_LEN,
                ))
            }
        } else {
            Some(random_username_from_principal(
                creator_principal,
                USERNAME_MAX_LEN,
            ))
        };

        // Handle follow relationships and profile info if profile was fetched
        if let Some(profile_response) = profile_result {
            match profile_response {
                Result3::Ok(profile_details) => {
                    post_details.user_follows_creator = profile_details.caller_follows_user;
                    post_details.creator_follows_user = profile_details.user_follows_caller;
                    post_details.creator_bio = profile_details.bio;
                    // Update propic_url if available from profile
                    if let Some(profile_pic) = profile_details.profile_picture_url {
                        if !profile_pic.is_empty() {
                            post_details.propic_url = profile_pic;
                        }
                    }
                }
                Result3::Err(e) => {
                    log::warn!(
                        "Failed to get profile details for creator {}: {}",
                        creator_principal,
                        e
                    );
                }
            }
        }

        Ok(Some(post_details))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_post_details_with_creator_info_v1(
        &self,
        user_canister: Principal,
        post_id: String,
        creator_principal: Principal,
        nsfw_probability: Option<f32>,
    ) -> Result<Option<PostDetails>> {
        // Fetch post details with NSFW info, creator metadata, and profile details concurrently
        let (post_details_result, creator_meta, profile_result) = try_join!(
            async {
                let details = self
                    .get_post_details_with_nsfw_info(
                        user_canister,
                        post_id.clone(),
                        nsfw_probability,
                    )
                    .await?;
                Ok::<_, Error>(details)
            },
            async {
                let meta = self
                    .metadata_client
                    .get_user_metadata_v2(creator_principal.to_text())
                    .await?;
                Ok::<_, Error>(meta)
            },
            async {
                let service_canister = self.user_info_service().await;
                let profile_details = service_canister
                    .get_profile_details_v_4(creator_principal)
                    .await?;
                Ok::<_, Error>(Some(profile_details))
            }
        )?;

        let Some(mut post_details) = post_details_result else {
            log::error!(
                "Post details not found for canister {} and post ID {},  skipping",
                user_canister,
                post_id
            );
            return Ok(None);
        };

        // Handle username: use metadata if available, otherwise generate
        post_details.username = if let Some(meta) = creator_meta {
            let username = meta.user_name;
            if !username.is_empty() {
                Some(username)
            } else {
                log::error!(
                    "Creator {} has empty username in metadata, generating fallback",
                    creator_principal
                );
                Some(random_username_from_principal(
                    creator_principal,
                    USERNAME_MAX_LEN,
                ))
            }
        } else {
            log::error!(
                "Failed to fetch metadata for creator {}, generating fallback username",
                creator_principal
            );
            Some(random_username_from_principal(
                creator_principal,
                USERNAME_MAX_LEN,
            ))
        };

        // Handle follow relationships and profile info if profile was fetched
        if let Some(profile_response) = profile_result {
            match profile_response {
                Result3::Ok(profile_details) => {
                    post_details.user_follows_creator = profile_details.caller_follows_user;
                    post_details.creator_follows_user = profile_details.user_follows_caller;
                    post_details.creator_bio = profile_details.bio;
                    // Update propic_url if available from profile
                    if let Some(profile_pic) = profile_details.profile_picture_url {
                        if !profile_pic.is_empty() {
                            post_details.propic_url = profile_pic;
                        }
                    }
                }
                Result3::Err(e) => {
                    log::error!(
                        "Failed to get profile details for creator {}: {}",
                        creator_principal,
                        e
                    );
                }
            }
        }

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
                    Result5::Ok(val) => Ok(val),
                    Result5::Err(err) => Err(crate::Error::YralCanister(format!("{err:?}"))),
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
