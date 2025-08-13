use crate::types::{MLFeedCacheHistoryItem, PostItem};
use crate::types_v2::{BufferItemV2, MLFeedCacheHistoryItemV2};
use crate::types_v3::{BufferItemV3, MLFeedCacheHistoryItemV3, PlainPostItemV3};
use ::types::post::{PostItemV2, PostItemV3};
use redis::{from_redis_value, RedisResult, Value};
use serde_json;

/// Custom deserializer for PostItemV2 that can handle both u64 and String post_ids
/// If post_id is a String, it attempts to parse it as u64, skipping if invalid
pub fn deserialize_post_item_v2_resilient(value: &Value) -> RedisResult<Option<PostItemV2>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Check if post_id is a string or number
                    if let Some(post_id_value) = json.get("post_id") {
                        let post_id = if let Some(num) = post_id_value.as_u64() {
                            num
                        } else if let Some(s) = post_id_value.as_str() {
                            // Try to parse string as u64
                            match s.parse::<u64>() {
                                Ok(num) => num,
                                Err(_) => {
                                    // Cannot parse as u64, skip this item
                                    return Ok(None);
                                }
                            }
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        };

                        // Reconstruct the item with parsed post_id
                        let item = PostItemV2 {
                            publisher_user_id: json
                                .get("publisher_user_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            canister_id: json
                                .get("canister_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            post_id,
                            video_id: json
                                .get("video_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            is_nsfw: json
                                .get("is_nsfw")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        };
                        Ok(Some(item))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<PostItemV2>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<PostItemV2>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for MLFeedCacheHistoryItemV2 that can handle both u64 and String post_ids
pub fn deserialize_history_item_v2_resilient(
    value: &Value,
) -> RedisResult<Option<MLFeedCacheHistoryItemV2>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Check if post_id is a string or number
                    if let Some(post_id_value) = json.get("post_id") {
                        let post_id = if let Some(num) = post_id_value.as_u64() {
                            num
                        } else if let Some(s) = post_id_value.as_str() {
                            // Try to parse string as u64
                            match s.parse::<u64>() {
                                Ok(num) => num,
                                Err(_) => {
                                    // Cannot parse as u64, skip this item
                                    return Ok(None);
                                }
                            }
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        };

                        // Parse timestamp
                        let timestamp = if let Some(ts) = json.get("timestamp") {
                            if let Some(secs) = ts.get("secs_since_epoch").and_then(|v| v.as_u64())
                            {
                                std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)
                            } else {
                                std::time::SystemTime::now()
                            }
                        } else {
                            std::time::SystemTime::now()
                        };

                        // Reconstruct the item with parsed post_id
                        let item = MLFeedCacheHistoryItemV2 {
                            publisher_user_id: json
                                .get("publisher_user_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            canister_id: json
                                .get("canister_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            post_id,
                            video_id: json
                                .get("video_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            item_type: json
                                .get("item_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            timestamp,
                            percent_watched: json
                                .get("percent_watched")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as f32,
                        };
                        Ok(Some(item))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<MLFeedCacheHistoryItemV2>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<MLFeedCacheHistoryItemV2>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for BufferItemV2 that can handle both u64 and String post_ids
pub fn deserialize_buffer_item_v2_resilient(value: &Value) -> RedisResult<Option<BufferItemV2>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Check if post_id is a string or number
                    if let Some(post_id_value) = json.get("post_id") {
                        let post_id = if let Some(num) = post_id_value.as_u64() {
                            num
                        } else if let Some(s) = post_id_value.as_str() {
                            // Try to parse string as u64
                            match s.parse::<u64>() {
                                Ok(num) => num,
                                Err(_) => {
                                    // Cannot parse as u64, skip this item
                                    return Ok(None);
                                }
                            }
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        };

                        // Parse timestamp
                        let timestamp = if let Some(ts) = json.get("timestamp") {
                            if let Some(secs) = ts.get("secs_since_epoch").and_then(|v| v.as_u64())
                            {
                                std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)
                            } else {
                                std::time::SystemTime::now()
                            }
                        } else {
                            std::time::SystemTime::now()
                        };

                        // Reconstruct the item with parsed post_id
                        let item = BufferItemV2 {
                            publisher_user_id: json
                                .get("publisher_user_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            post_id,
                            video_id: json
                                .get("video_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            item_type: json
                                .get("item_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            percent_watched: json
                                .get("percent_watched")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as f32,
                            user_id: json
                                .get("user_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            timestamp,
                        };
                        Ok(Some(item))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<BufferItemV2>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<BufferItemV2>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for PostItemV3 that can handle both u64 and String post_ids
/// Converts u64 post_ids to String for V3 compatibility
pub fn deserialize_post_item_v3_resilient(value: &Value) -> RedisResult<Option<PostItemV3>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Get post_id and convert to String
                    let post_id = if let Some(post_id_value) = json.get("post_id") {
                        if let Some(s) = post_id_value.as_str() {
                            s.to_string()
                        } else if let Some(num) = post_id_value.as_u64() {
                            num.to_string()
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    };

                    // Reconstruct the item with String post_id
                    let item = PostItemV3 {
                        publisher_user_id: json
                            .get("publisher_user_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        canister_id: json
                            .get("canister_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        post_id,
                        video_id: json
                            .get("video_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        is_nsfw: json
                            .get("is_nsfw")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    };
                    Ok(Some(item))
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<PostItemV3>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<PostItemV3>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for MLFeedCacheHistoryItemV3 that can handle both u64 and String post_ids
pub fn deserialize_history_item_v3_resilient(
    value: &Value,
) -> RedisResult<Option<MLFeedCacheHistoryItemV3>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Get post_id and convert to String
                    let post_id = if let Some(post_id_value) = json.get("post_id") {
                        if let Some(s) = post_id_value.as_str() {
                            s.to_string()
                        } else if let Some(num) = post_id_value.as_u64() {
                            num.to_string()
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    };

                    // Parse timestamp
                    let timestamp = if let Some(ts) = json.get("timestamp") {
                        if let Some(ts_obj) = ts.as_object() {
                            if let (Some(secs), Some(nanos)) = (
                                ts_obj.get("secs_since_epoch").and_then(|v| v.as_u64()),
                                ts_obj.get("nanos_since_epoch").and_then(|v| v.as_u64()),
                            ) {
                                std::time::SystemTime::UNIX_EPOCH
                                    + std::time::Duration::new(secs, nanos as u32)
                            } else {
                                std::time::SystemTime::now()
                            }
                        } else {
                            std::time::SystemTime::now()
                        }
                    } else {
                        std::time::SystemTime::now()
                    };

                    // Reconstruct the item with String post_id
                    let item = MLFeedCacheHistoryItemV3 {
                        publisher_user_id: json
                            .get("publisher_user_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        canister_id: json
                            .get("canister_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        post_id,
                        video_id: json
                            .get("video_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        item_type: json
                            .get("item_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        timestamp,
                        percent_watched: json
                            .get("percent_watched")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32,
                    };
                    Ok(Some(item))
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<MLFeedCacheHistoryItemV3>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<MLFeedCacheHistoryItemV3>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for BufferItemV3 that can handle both u64 and String post_ids
pub fn deserialize_buffer_item_v3_resilient(value: &Value) -> RedisResult<Option<BufferItemV3>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Get post_id and convert to String
                    let post_id = if let Some(post_id_value) = json.get("post_id") {
                        if let Some(s) = post_id_value.as_str() {
                            s.to_string()
                        } else if let Some(num) = post_id_value.as_u64() {
                            num.to_string()
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    };

                    // Parse timestamp
                    let timestamp = if let Some(ts) = json.get("timestamp") {
                        if let Some(ts_obj) = ts.as_object() {
                            if let (Some(secs), Some(nanos)) = (
                                ts_obj.get("secs_since_epoch").and_then(|v| v.as_u64()),
                                ts_obj.get("nanos_since_epoch").and_then(|v| v.as_u64()),
                            ) {
                                std::time::SystemTime::UNIX_EPOCH
                                    + std::time::Duration::new(secs, nanos as u32)
                            } else {
                                std::time::SystemTime::now()
                            }
                        } else {
                            std::time::SystemTime::now()
                        }
                    } else {
                        std::time::SystemTime::now()
                    };

                    // Reconstruct the item with String post_id
                    let item = BufferItemV3 {
                        publisher_user_id: json
                            .get("publisher_user_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        post_id,
                        video_id: json
                            .get("video_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        item_type: json
                            .get("item_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        percent_watched: json
                            .get("percent_watched")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32,
                        user_id: json
                            .get("user_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        timestamp,
                    };
                    Ok(Some(item))
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<BufferItemV3>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<BufferItemV3>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for legacy V1 PostItem that can handle both u64 and String post_ids
pub fn deserialize_post_item_v1_resilient(value: &Value) -> RedisResult<Option<PostItem>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Check if post_id is a string or number
                    if let Some(post_id_value) = json.get("post_id") {
                        let post_id = if let Some(num) = post_id_value.as_u64() {
                            num
                        } else if let Some(s) = post_id_value.as_str() {
                            // Try to parse string as u64
                            match s.parse::<u64>() {
                                Ok(num) => num,
                                Err(_) => {
                                    // Cannot parse as u64, skip this item
                                    return Ok(None);
                                }
                            }
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        };

                        // Reconstruct the item with parsed post_id
                        let item = PostItem {
                            canister_id: json
                                .get("canister_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            post_id,
                            video_id: json
                                .get("video_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            nsfw_probability: json
                                .get("nsfw_probability")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0)
                                as f32,
                        };
                        Ok(Some(item))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<PostItem>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<PostItem>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for legacy V1 MLFeedCacheHistoryItem that can handle both u64 and String post_ids
pub fn deserialize_history_item_v1_resilient(
    value: &Value,
) -> RedisResult<Option<MLFeedCacheHistoryItem>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Check if post_id is a string or number
                    if let Some(post_id_value) = json.get("post_id") {
                        let post_id = if let Some(num) = post_id_value.as_u64() {
                            num
                        } else if let Some(s) = post_id_value.as_str() {
                            // Try to parse string as u64
                            match s.parse::<u64>() {
                                Ok(num) => num,
                                Err(_) => {
                                    // Cannot parse as u64, skip this item
                                    return Ok(None);
                                }
                            }
                        } else {
                            // Invalid post_id type
                            return Ok(None);
                        };

                        // Parse timestamp
                        let timestamp = if let Some(ts) = json.get("timestamp") {
                            if let Some(secs) = ts.get("secs_since_epoch").and_then(|v| v.as_u64())
                            {
                                std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)
                            } else {
                                std::time::SystemTime::now()
                            }
                        } else {
                            std::time::SystemTime::now()
                        };

                        // Reconstruct the item with parsed post_id
                        let item = MLFeedCacheHistoryItem {
                            canister_id: json
                                .get("canister_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            post_id,
                            video_id: json
                                .get("video_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            nsfw_probability: json
                                .get("nsfw_probability")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0)
                                as f32,
                            item_type: json
                                .get("item_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            timestamp,
                            percent_watched: json
                                .get("percent_watched")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as f32,
                        };
                        Ok(Some(item))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<MLFeedCacheHistoryItem>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<MLFeedCacheHistoryItem>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Custom deserializer for PlainPostItemV3 that handles both V2 and V3 plain items
/// V2 and V3 plain items both only have video_id
pub fn deserialize_plain_post_item_v3_resilient(
    value: &Value,
) -> RedisResult<Option<PlainPostItemV3>> {
    match value {
        Value::BulkString(bytes) => {
            // Try to deserialize as JSON
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => {
                    // Get video_id
                    let video_id = if let Some(video_id_value) = json.get("video_id") {
                        if let Some(s) = video_id_value.as_str() {
                            s.to_string()
                        } else {
                            // Invalid video_id type
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    };

                    // Reconstruct the item with video_id only
                    let item = PlainPostItemV3 { video_id };
                    Ok(Some(item))
                }
                Err(_) => {
                    // Try standard deserialization
                    match from_redis_value::<PlainPostItemV3>(value) {
                        Ok(item) => Ok(Some(item)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        _ => {
            // Try standard deserialization
            match from_redis_value::<PlainPostItemV3>(value) {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            }
        }
    }
}
