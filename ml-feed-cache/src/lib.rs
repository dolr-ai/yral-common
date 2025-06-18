use std::time::{SystemTime, UNIX_EPOCH};

use consts::{
    MAX_GLOBAL_CACHE_LEN, MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN, MAX_SUCCESS_HISTORY_CACHE_LEN,
    MAX_USER_CACHE_LEN, MAX_WATCH_HISTORY_CACHE_LEN, USER_HOTORNOT_BUFFER_KEY,
};
use redis::AsyncCommands;
use types::{get_history_item_score, BufferItem, MLFeedCacheHistoryItem, PlainPostItem, PostItem};

pub mod consts;
pub mod types;

pub type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[derive(Clone)]
pub struct MLFeedCacheState {
    pub redis_pool: RedisPool,
}

pub async fn init_redis() -> RedisPool {
    let redis_url =
        std::env::var("ML_FEED_CACHE_REDIS_URL").expect("ML_FEED_CACHE_REDIS_URL must be set");

    let manager = bb8_redis::RedisConnectionManager::new(redis_url.clone())
        .expect("failed to open connection to redis");
    RedisPool::builder().build(manager).await.unwrap()
}

impl MLFeedCacheState {
    pub async fn new() -> Self {
        let redis_pool = init_redis().await;
        Self { redis_pool }
    }

    pub async fn add_user_watch_history_items(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| (get_history_item_score(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItem, ()>(key, chunk)
                .await?;
        }

        // get num items in the list
        let num_items = conn.zcard::<&str, u64>(key).await?;

        // if num items is greater than MAX_WATCH_HISTORY_CACHE_LEN, remove the oldest items till len is MAX_WATCH_HISTORY_CACHE_LEN without while loop
        if num_items > MAX_WATCH_HISTORY_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_WATCH_HISTORY_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn add_user_success_history_items(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| (get_history_item_score(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItem, ()>(key, chunk)
                .await?;
        }

        // get num items in the list
        let num_items = conn.zcard::<&str, u64>(key).await?;

        if num_items > MAX_SUCCESS_HISTORY_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_SUCCESS_HISTORY_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn get_history_items(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<MLFeedCacheHistoryItem>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrevrange::<&str, Vec<MLFeedCacheHistoryItem>>(key, start as isize, end as isize)
            .await?;

        Ok(items)
    }

    pub async fn get_history_items_len(&self, key: &str) -> Result<u64, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();
        let num_items = conn.zcard::<&str, u64>(key).await?;
        Ok(num_items)
    }

    pub async fn add_user_history_plain_items(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| {
                (
                    item.timestamp
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    PlainPostItem {
                        canister_id: item.canister_id.clone(),
                        post_id: item.post_id,
                    },
                )
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, u64, PlainPostItem, ()>(key, chunk)
                .await?;
        }

        // get num items in the list
        let num_items = conn.zcard::<&str, u64>(key).await?;

        // if num items is greater than MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN, remove the oldest items till len is MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN without while loop
        if num_items > MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn is_user_history_plain_item_exists(
        &self,
        key: &str,
        item: PlainPostItem,
    ) -> Result<bool, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let res = conn
            .zscore::<&str, PlainPostItem, Option<f64>>(key, item)
            .await?;

        Ok(res.is_some())
    }

    pub async fn add_user_cache_items(
        &self,
        key: &str,
        items: Vec<PostItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let items = items
            .iter()
            .map(|item| (timestamp, item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, PostItem, ()>(key, chunk)
                .await?;
        }

        // get num items in the list
        let num_items = conn.zcard::<&str, u64>(key).await?;

        if num_items > MAX_USER_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(key, 0, (num_items - MAX_USER_CACHE_LEN - 1) as isize)
                .await?;
        }

        Ok(())
    }

    pub async fn add_global_cache_items(
        &self,
        key: &str,
        items: Vec<PostItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let items = items
            .iter()
            .map(|item| (timestamp, item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, PostItem, ()>(key, chunk)
                .await?;
        }

        // get num items in the list
        let num_items = conn.zcard::<&str, u64>(key).await?;

        if num_items > MAX_GLOBAL_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - MAX_GLOBAL_CACHE_LEN - 1) as isize,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn get_cache_items(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<PostItem>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrevrange::<&str, Vec<PostItem>>(key, start as isize, end as isize)
            .await?;

        Ok(items)
    }

    pub async fn get_cache_items_len(&self, key: &str) -> Result<u64, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();
        let num_items = conn.zcard::<&str, u64>(key).await?;
        Ok(num_items)
    }

    pub async fn add_user_buffer_items(&self, items: Vec<BufferItem>) -> Result<(), anyhow::Error> {
        self.add_user_buffer_items_impl(USER_HOTORNOT_BUFFER_KEY, items)
            .await
    }

    pub async fn add_user_buffer_items_impl(
        &self,
        key: &str,
        items: Vec<BufferItem>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| {
                (
                    item.timestamp
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    item.clone(),
                )
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, u64, BufferItem, ()>(key, chunk)
                .await?;
        }

        Ok(())
    }

    pub async fn get_user_buffer_items_by_timestamp(
        &self,
        timestamp: u64,
    ) -> Result<Vec<BufferItem>, anyhow::Error> {
        self.get_user_buffer_items_by_timestamp_impl(USER_HOTORNOT_BUFFER_KEY, timestamp)
            .await
    }

    pub async fn get_user_buffer_items_by_timestamp_impl(
        &self,
        key: &str,
        timestamp_secs: u64,
    ) -> Result<Vec<BufferItem>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrangebyscore::<&str, u64, u64, Vec<BufferItem>>(key, 0, timestamp_secs)
            .await?;

        Ok(items)
    }

    pub async fn remove_user_buffer_items_by_timestamp(
        &self,
        timestamp_secs: u64,
    ) -> Result<u64, anyhow::Error> {
        self.remove_user_buffer_items_by_timestamp_impl(USER_HOTORNOT_BUFFER_KEY, timestamp_secs)
            .await
    }

    pub async fn remove_user_buffer_items_by_timestamp_impl(
        &self,
        key: &str,
        timestamp_secs: u64,
    ) -> Result<u64, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let res = conn
            .zrembyscore::<&str, u64, u64, u64>(key, 0, timestamp_secs)
            .await?;

        Ok(res)
    }

    pub async fn delete_user_caches(&self, key: &str) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // All user cache suffixes
        #[allow(clippy::useless_vec)]
        let suffixes = vec![
            consts::USER_WATCH_HISTORY_CLEAN_SUFFIX,
            consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX,
            consts::USER_WATCH_HISTORY_NSFW_SUFFIX,
            consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX,
            consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX,
            consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX,
            consts::USER_CACHE_CLEAN_SUFFIX,
            consts::USER_CACHE_NSFW_SUFFIX,
            consts::USER_CACHE_MIXED_SUFFIX,
        ];

        // Build all keys with suffixes
        let keys: Vec<String> = suffixes
            .iter()
            .map(|suffix| format!("{}{}", key, suffix))
            .collect();

        // Delete all keys in one statement
        conn.del::<Vec<String>, ()>(keys).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_add_user_watch_history_items() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key").await;
        assert!(_res.is_ok());

        let _res = conn.del::<&str, ()>("test_key_plain").await;
        assert!(_res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..MAX_WATCH_HISTORY_CACHE_LEN + 10 {
            items.push(MLFeedCacheHistoryItem {
                video_id: format!("test_video_id{i}"),
                item_type: "video_viewed".to_string(),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                nsfw_probability: 0.0,
                timestamp: SystemTime::now(),
                percent_watched: i as f32 / 100.0,
            });
        }

        let res = state
            .add_user_watch_history_items("test_key", items.clone())
            .await;
        assert!(res.is_ok());

        // add plain post items
        let res = state
            .add_user_history_plain_items("test_key_plain", items.clone())
            .await;
        assert!(res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, MAX_WATCH_HISTORY_CACHE_LEN);

        let num_items_plain = conn.zcard::<&str, u64>("test_key_plain").await.unwrap();
        assert_eq!(num_items_plain, MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN);

        let items = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItem, f64)>>("test_key", 0, 4)
            .await
            .unwrap();
        assert_eq!(items.len(), 5);

        // print the items
        for item in items {
            println!("{item:?}");
        }

        // check if the plain item exists
        let res = state
            .is_user_history_plain_item_exists(
                "test_key_plain",
                PlainPostItem {
                    canister_id: "test_canister_id".to_string(),
                    post_id: MAX_WATCH_HISTORY_CACHE_LEN + 10 - 1,
                },
            )
            .await;
        assert!(res.is_ok());
        assert!(res.unwrap());

        // check if the plain item does not exist
        let res = state
            .is_user_history_plain_item_exists(
                "test_key_plain",
                PlainPostItem {
                    canister_id: "test_canister_id".to_string(),
                    post_id: MAX_WATCH_HISTORY_CACHE_LEN + 10 + 1,
                },
            )
            .await;
        assert!(res.is_ok());
        assert!(!res.unwrap());
    }

    #[tokio::test]
    async fn test_add_user_success_history_items() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key").await;
        assert!(_res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..MAX_SUCCESS_HISTORY_CACHE_LEN + 100 {
            items.push(MLFeedCacheHistoryItem {
                video_id: format!("test_video_id{i}"),
                item_type: "like_video".to_string(),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                nsfw_probability: 0.0,
                timestamp: SystemTime::now() + Duration::from_secs(i * 100_u64),
                percent_watched: 0.0,
            });
        }

        let res = state
            .add_user_success_history_items("test_key", items)
            .await;
        assert!(res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, MAX_SUCCESS_HISTORY_CACHE_LEN);

        let items = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItem, f64)>>("test_key", 0, 4)
            .await
            .unwrap();
        assert_eq!(items.len(), 5);

        // print the items
        for item in items {
            println!("{item:?}");
        }
    }

    #[tokio::test]
    async fn test_add_user_buffer_items() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();

        let _res = conn.del::<&str, ()>("test_key").await;
        assert!(_res.is_ok());

        let _res = conn.del::<&str, ()>(USER_HOTORNOT_BUFFER_KEY).await;
        assert!(_res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..100 {
            items.push(BufferItem {
                publisher_canister_id: "test_publisher_canister_id".to_string(),
                user_canister_id: "test_user_canister_id".to_string(),
                post_id: i,
                video_id: format!("test_video_id{i}"),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now() + Duration::from_secs(i * 100_u64),
                percent_watched: 50.0,
            });
        }

        let res = state
            .add_user_buffer_items_impl("test_key", items.clone())
            .await;
        assert!(res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, 100);

        let res_items = conn
            .zrevrange_withscores::<&str, Vec<(BufferItem, u64)>>("test_key", 0, 4)
            .await
            .unwrap();
        assert_eq!(res_items.len(), 5);

        // print the items
        for item in res_items.iter() {
            println!("{item:?}");
        }

        // check get_user_buffer_items_by_timestamp
        let timestamp = items[4].timestamp;
        let timestamp_secs = timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let items = state
            .get_user_buffer_items_by_timestamp_impl("test_key", timestamp_secs)
            .await
            .unwrap();
        assert_eq!(items.len(), 5);

        // print the items
        for item in items.iter() {
            println!("{item:?}");
        }

        // remove the items
        let res = state
            .remove_user_buffer_items_by_timestamp_impl("test_key", timestamp_secs)
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 5);

        let num_items = conn.zcard::<&str, u64>("test_key").await.unwrap();
        assert_eq!(num_items, 95);
    }

    #[tokio::test]
    async fn test_delete_user_caches() {
        let state = MLFeedCacheState::new().await;
        let mut conn = state.redis_pool.get().await.unwrap();

        let test_base_key = "test_user_delete";

        // Create some test data for each cache type
        let test_items = vec![
            PostItem {
                canister_id: "test_canister".to_string(),
                post_id: 1,
                video_id: "test_video_1".to_string(),
                nsfw_probability: 0.1,
            },
            PostItem {
                canister_id: "test_canister".to_string(),
                post_id: 2,
                video_id: "test_video_2".to_string(),
                nsfw_probability: 0.2,
            },
        ];

        let history_items = vec![
            MLFeedCacheHistoryItem {
                video_id: "test_video_1".to_string(),
                item_type: "video_viewed".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 1,
                nsfw_probability: 0.0,
                timestamp: SystemTime::now(),
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItem {
                video_id: "test_video_2".to_string(),
                item_type: "like_video".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 2,
                nsfw_probability: 0.0,
                timestamp: SystemTime::now(),
                percent_watched: 100.0,
            },
        ];

        // Add data to various cache types
        state
            .add_user_cache_items(
                &format!("{}{}", test_base_key, consts::USER_CACHE_CLEAN_SUFFIX),
                test_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_cache_items(
                &format!("{}{}", test_base_key, consts::USER_CACHE_NSFW_SUFFIX),
                test_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_cache_items(
                &format!("{}{}", test_base_key, consts::USER_CACHE_MIXED_SUFFIX),
                test_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_watch_history_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_CLEAN_SUFFIX
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_watch_history_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_NSFW_SUFFIX
                ),
                history_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_success_history_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_success_history_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX
                ),
                history_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_history_plain_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_history_plain_items(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX
                ),
                history_items,
            )
            .await
            .unwrap();

        // Verify data exists
        let cache_clean_len = conn
            .zcard::<&str, u64>(&format!(
                "{}{}",
                test_base_key,
                consts::USER_CACHE_CLEAN_SUFFIX
            ))
            .await
            .unwrap();
        assert_eq!(cache_clean_len, 2);

        let watch_clean_len = conn
            .zcard::<&str, u64>(&format!(
                "{}{}",
                test_base_key,
                consts::USER_WATCH_HISTORY_CLEAN_SUFFIX
            ))
            .await
            .unwrap();
        assert_eq!(watch_clean_len, 2);

        // Delete all user caches
        state.delete_user_caches(test_base_key).await.unwrap();

        // Verify all caches are deleted
        let suffixes = vec![
            consts::USER_WATCH_HISTORY_CLEAN_SUFFIX,
            consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX,
            consts::USER_WATCH_HISTORY_NSFW_SUFFIX,
            consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX,
            consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX,
            consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX,
            consts::USER_CACHE_CLEAN_SUFFIX,
            consts::USER_CACHE_NSFW_SUFFIX,
            consts::USER_CACHE_MIXED_SUFFIX,
        ];

        for suffix in suffixes {
            let full_key = format!("{}{}", test_base_key, suffix);
            let exists = conn.exists::<&str, bool>(&full_key).await.unwrap();
            assert!(!exists, "Key {} should not exist", full_key);
        }
    }
}
