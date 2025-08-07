use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use candid::Principal;
use canisters_client::user_post_service::PostDetailsForFrontend as PostServicePostDetailsForFrontend;
use canisters_client::{
    individual_user_template::{Post, PostDetailsForFrontend, PostStatus},
    user_info_service::UserProfileDetailsForFrontendV3,
};
use serde::{Deserialize, Serialize};
use web_time::Duration;

use crate::{Canisters, Result};

use super::profile::propic_from_principal;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PostDetails {
    pub canister_id: Principal, // canister id of the publishing canister.
    pub post_id: String,
    pub uid: String,
    pub description: String,
    pub views: u64,
    pub likes: u64,
    pub display_name: String,
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

    pub fn from_service_post(
        canister_id: Principal,
        user_details: UserProfileDetailsForFrontendV3,
        post_details: PostServicePostDetailsForFrontend,
    ) -> Self {
        Self {
            canister_id,
            post_id: post_details.id,
            uid: post_details.video_uid,
            description: post_details.description,
            views: post_details.total_view_count,
            likes: post_details.like_count,
            display_name: user_details.principal_id.to_text(),
            propic_url: user_details.profile_picture_url.unwrap_or_else(|| {
                propic_from_principal(post_details.created_by_user_principal_id)
            }),
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
            display_name: details
                .created_by_display_name
                .or(details.created_by_unique_user_name)
                .unwrap_or_else(|| details.created_by_user_principal_id.to_text()),
            propic_url: details
                .created_by_profile_photo_url
                .unwrap_or_else(|| propic_from_principal(details.created_by_user_principal_id)),
            liked_by_user: authenticated.then_some(details.liked_by_me),
            poster_principal: details.created_by_user_principal_id,
            hastags: details.hashtags,
            is_nsfw: nsfw_probability > 0.5,
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
}

impl<const A: bool> Canisters<A> {
    pub async fn get_post_details(
        &self,
        user_canister: Principal,
        post_id: u64,
    ) -> Result<Option<PostDetails>> {
        self.get_post_details_with_nsfw_info(user_canister, post_id, 0.0)
            .await
    }

    pub async fn get_post_details_with_nsfw_info(
        &self,
        user_canister: Principal,
        post_id: u64,
        nsfw_probability: f32,
    ) -> Result<Option<PostDetails>> {
        let post_creator_can = self.individual_user(user_canister).await;
        let mut post_details = match post_creator_can
            .get_individual_post_details_by_id(post_id)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "failed to get post details for {user_canister} {post_id}: {e}, skipping"
                );
                return Ok(None);
            }
        };

        // TODO: temporary patch in frontend to not show banned videos, to be removed later after NSFW tagging
        if matches!(post_details.status, PostStatus::BannedDueToUserReporting) {
            return Ok(None);
        }

        let post_uuid = &post_details.video_uid;
        let req_url = format!(
            "https://customer-2p3jflss4r4hmpnz.cloudflarestream.com/{post_uuid}/manifest/video.m3u8",
        );
        let res = reqwest::Client::default().head(req_url).send().await;
        if res.is_err() || (res.is_ok() && res.unwrap().status() != 200) {
            return Ok(None);
        }

        let creator_principal = post_details.created_by_user_principal_id;
        let creator_meta = self
            .metadata_client
            .get_user_metadata_v2(creator_principal.to_text())
            .await?;
        post_details.created_by_unique_user_name =
            creator_meta.map(|m| m.user_name).filter(|s| !s.is_empty());

        Ok(Some(PostDetails::from_canister_post_with_nsfw_info(
            A,
            user_canister,
            post_details,
            nsfw_probability,
        )))
    }

    pub async fn post_like_info(
        &self,
        post_canister: Principal,
        post_id: u64,
    ) -> Result<(bool, u64)> {
        let individual = self.individual_user(post_canister).await;
        let post = individual
            .get_individual_post_details_by_id(post_id)
            .await?;
        Ok((post.liked_by_me, post.like_count))
    }
}
