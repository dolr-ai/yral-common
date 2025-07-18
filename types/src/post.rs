use ic_agent::export::Principal;
#[cfg(feature = "redis")]
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostItem {
    pub canister_id: Principal,
    pub post_id: u64,
    pub video_id: String,
    pub nsfw_probability: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeedRequest {
    pub canister_id: Principal,
    pub filter_results: Vec<PostItem>,
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeedResponse {
    pub posts: Vec<PostItem>,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
#[cfg_attr(feature = "redis", derive(ToRedisArgs, FromRedisValue))]
pub struct PostItemV2 {
    pub publisher_user_id: String,
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub is_nsfw: bool,
}

impl Eq for PostItemV2 {}

impl PartialEq for PostItemV2 {
    fn eq(&self, other: &Self) -> bool {
        self.video_id == other.video_id
    }
}

impl Hash for PostItemV2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.video_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FeedRequestV2 {
    #[schema(value_type = String)]
    pub user_id: Principal,
    pub filter_results: Vec<PostItemV2>,
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FeedResponseV2 {
    pub posts: Vec<PostItemV2>,
}
