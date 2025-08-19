use ic_agent::export::Principal;
#[cfg(feature = "redis")]
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Deserializer, Serialize};
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

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
#[cfg_attr(feature = "redis", derive(ToRedisArgs, FromRedisValue))]
pub struct PostItemV3 {
    pub publisher_user_id: String,
    pub canister_id: String,
    pub post_id: String,
    pub video_id: String,
    #[serde(deserialize_with = "is_nsfw", rename = "nsfw_probability")]
    pub is_nsfw: bool,
}

fn is_nsfw<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let probablity = f64::deserialize(d)?;

    Ok(probablity > 0.4)
}

impl Eq for PostItemV3 {}

impl PartialEq for PostItemV3 {
    fn eq(&self, other: &Self) -> bool {
        self.video_id == other.video_id
    }
}

impl Hash for PostItemV3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.video_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FeedRequestV3 {
    #[schema(value_type = String)]
    pub user_id: Principal,
    /// Video IDs to exclude from recommendations
    pub exclude_items: Vec<String>,
    /// Whether to include NSFW content in recommendations
    pub nsfw_label: bool,
    /// Number of results to return
    pub num_results: u32,
    /// IP address for geolocation
    pub ip_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FeedResponseV3 {
    pub posts: Vec<PostItemV3>,
}
