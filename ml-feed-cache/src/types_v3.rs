use std::{hash::Hash, time::SystemTime};

use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct MLFeedCacheHistoryItemV3 {
    pub publisher_user_id: String,
    pub canister_id: String,
    pub post_id: String, // Changed from u64 to String
    pub video_id: String,
    pub item_type: String,
    pub timestamp: SystemTime,
    pub percent_watched: f32,
}

pub fn get_history_item_score(item: &MLFeedCacheHistoryItemV3) -> f64 {
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

#[derive(
    Serialize, Deserialize, Clone, ToSchema, Debug, ToRedisArgs, FromRedisValue, Eq, PartialEq, Hash,
)]
pub struct PlainPostItemV3 {
    pub video_id: String,
    pub post_id: String, // Added post_id as String (V2 only had video_id)
}

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct BufferItemV3 {
    pub publisher_user_id: String,
    pub post_id: String, // Changed from u64 to String
    pub video_id: String,
    pub item_type: String,
    pub percent_watched: f32,
    pub user_id: String,
    pub timestamp: SystemTime,
}