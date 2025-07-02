use std::{
    hash::{Hash, Hasher},
    time::SystemTime,
};

use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct MLFeedCacheHistoryItem {
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub nsfw_probability: f32,
    pub item_type: String,
    pub timestamp: SystemTime,
    pub percent_watched: f32,
}

pub fn get_history_item_score(item: &MLFeedCacheHistoryItem) -> f64 {
    // Convert timestamp to seconds since epoch
    let timestamp_secs = item
        .timestamp
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    // TODO: Add a better scoring system. For now timestamp is the most important

    let item_type_score = if item.item_type == "like_video" {
        100.0
    } else {
        0.0
    };

    let percent_watched_score = (item.percent_watched * 100.0) as f64;

    timestamp_secs + item_type_score + percent_watched_score
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug, ToRedisArgs, FromRedisValue)]
pub struct PostItem {
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub nsfw_probability: f32,
}

impl Eq for PostItem {}

impl PartialEq for PostItem {
    fn eq(&self, other: &Self) -> bool {
        self.canister_id == other.canister_id && self.post_id == other.post_id
    }
}

impl Hash for PostItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canister_id.hash(state);
        self.post_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug, ToRedisArgs, FromRedisValue)]
pub struct PlainPostItem {
    pub canister_id: String,
    pub post_id: u64,
}

impl Eq for PlainPostItem {}

impl PartialEq for PlainPostItem {
    fn eq(&self, other: &Self) -> bool {
        self.canister_id == other.canister_id && self.post_id == other.post_id
    }
}

impl Hash for PlainPostItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canister_id.hash(state);
        self.post_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedRequest {
    pub canister_id: String,
    pub filter_results: Vec<PostItem>,
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedResponse {
    pub posts: Vec<PostItem>,
}

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct BufferItem {
    pub publisher_canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub item_type: String,
    pub percent_watched: f32,
    pub user_canister_id: String,
    pub timestamp: SystemTime,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedRequestV2 {
    pub user_id: String,
    pub canister_id: String,
    pub filter_results: Vec<String>, // List of video IDs to filter results
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug, ToRedisArgs, FromRedisValue)]
pub struct PostItemV2 {
    pub publisher_user_id: String,
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub nsfw_probability: f32,
}

impl Eq for PostItemV2 {}

impl PartialEq for PostItemV2 {
    fn eq(&self, other: &Self) -> bool {
        self.publisher_user_id == other.publisher_user_id && self.post_id == other.post_id
    }
}

impl Hash for PostItemV2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.publisher_user_id.hash(state);
        self.post_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedResponseV2 {
    pub posts: Vec<PostItemV2>,
}
