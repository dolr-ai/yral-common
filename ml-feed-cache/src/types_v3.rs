use std::{hash::Hash, time::SystemTime};

use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(u64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct MLFeedCacheHistoryItemV3 {
    pub publisher_user_id: String,
    pub canister_id: String,
    #[serde(deserialize_with = "string_or_number")]
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
}

#[derive(Serialize, Deserialize, Clone, ToRedisArgs, FromRedisValue, Debug)]
pub struct BufferItemV3 {
    pub publisher_user_id: String,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String, // Changed from u64 to String
    pub video_id: String,
    pub item_type: String,
    pub percent_watched: f32,
    pub user_id: String,
    pub timestamp: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixed_type_deserialization_history_item() {
        // Test with numeric post_id
        let json_with_number = r#"{
            "publisher_user_id": "user123",
            "canister_id": "canister456",
            "post_id": 789,
            "video_id": "video111",
            "item_type": "like_video",
            "timestamp": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0},
            "percent_watched": 75.5
        }"#;

        let item: MLFeedCacheHistoryItemV3 = serde_json::from_str(json_with_number).unwrap();
        assert_eq!(item.post_id, "789");

        // Test with string post_id
        let json_with_string = r#"{
            "publisher_user_id": "user123",
            "canister_id": "canister456",
            "post_id": "789",
            "video_id": "video111",
            "item_type": "like_video",
            "timestamp": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0},
            "percent_watched": 75.5
        }"#;

        let item: MLFeedCacheHistoryItemV3 = serde_json::from_str(json_with_string).unwrap();
        assert_eq!(item.post_id, "789");
    }

    #[test]
    fn test_mixed_type_deserialization_buffer_item() {
        // Test with numeric post_id
        let json_with_number = r#"{
            "publisher_user_id": "publisher123",
            "post_id": 456,
            "video_id": "video789",
            "item_type": "video_watched",
            "percent_watched": 50.0,
            "user_id": "user999",
            "timestamp": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0}
        }"#;

        let item: BufferItemV3 = serde_json::from_str(json_with_number).unwrap();
        assert_eq!(item.post_id, "456");

        // Test with string post_id
        let json_with_string = r#"{
            "publisher_user_id": "publisher123",
            "post_id": "456",
            "video_id": "video789",
            "item_type": "video_watched",
            "percent_watched": 50.0,
            "user_id": "user999",
            "timestamp": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0}
        }"#;

        let item: BufferItemV3 = serde_json::from_str(json_with_string).unwrap();
        assert_eq!(item.post_id, "456");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let item = MLFeedCacheHistoryItemV3 {
            publisher_user_id: "user123".to_string(),
            canister_id: "canister456".to_string(),
            post_id: "789".to_string(),
            video_id: "video111".to_string(),
            item_type: "like_video".to_string(),
            timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1700000000),
            percent_watched: 75.5,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&item).unwrap();

        // Deserialize back
        let deserialized: MLFeedCacheHistoryItemV3 = serde_json::from_str(&json).unwrap();

        // Check equality (note: direct equality won't work due to SystemTime precision)
        assert_eq!(item.post_id, deserialized.post_id);
        assert_eq!(item.publisher_user_id, deserialized.publisher_user_id);
        assert_eq!(item.canister_id, deserialized.canister_id);
        assert_eq!(item.video_id, deserialized.video_id);
        assert_eq!(item.item_type, deserialized.item_type);
        assert_eq!(item.percent_watched, deserialized.percent_watched);
    }
}
