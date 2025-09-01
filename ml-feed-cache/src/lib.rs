use std::time::{SystemTime, UNIX_EPOCH};

use ::types::post::{PostItemV2, PostItemV3};
use consts::{
    MAX_GLOBAL_CACHE_LEN, MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN, MAX_SUCCESS_HISTORY_CACHE_LEN,
    MAX_USER_CACHE_LEN, MAX_WATCH_HISTORY_CACHE_LEN, USER_HOTORNOT_BUFFER_KEY,
    USER_HOTORNOT_BUFFER_KEY_V2,
};
use redis::AsyncCommands;
use types::{get_history_item_score, BufferItem, MLFeedCacheHistoryItem, PlainPostItem, PostItem};
use types_v2::{
    get_history_item_score as get_history_item_score_v2, BufferItemV2, MLFeedCacheHistoryItemV2,
    PlainPostItemV2,
};
use types_v3::{
    get_history_item_score as get_history_item_score_v3, BufferItemV3, MLFeedCacheHistoryItemV3,
    PlainPostItemV3,
};

pub mod consts;
pub mod types;
pub mod types_v2;
pub mod types_v3;

pub type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[derive(Clone)]
pub struct MLFeedCacheState {
    pub redis_pool: RedisPool,
    pub memory_store_pool: RedisPool,
}

pub async fn init_redis() -> RedisPool {
    let redis_url =
        std::env::var("ML_FEED_CACHE_REDIS_URL").expect("ML_FEED_CACHE_REDIS_URL must be set");

    let manager = bb8_redis::RedisConnectionManager::new(redis_url.clone())
        .expect("failed to open connection to redis");
    RedisPool::builder().build(manager).await.unwrap()
}

pub async fn init_memorystore() -> RedisPool {
    let memorystore_url = std::env::var("ML_FEED_CACHE_MEMORYSTORE_URL")
        .expect("ML_FEED_CACHE_MEMORYSTORE_URL must be set");

    let manager = bb8_redis::RedisConnectionManager::new(memorystore_url.clone())
        .expect("failed to open connection to memorystore_url");
    RedisPool::builder().build(manager).await.unwrap()
}

impl MLFeedCacheState {
    /// Helper method to update memory store pool asynchronously without blocking
    fn spawn_memory_store_update<F>(&self, key: &str, operation: F)
    where
        F: FnOnce(
                RedisPool,
                String,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), anyhow::Error>> + Send>,
            > + Send
            + 'static,
    {
        let memory_pool = self.memory_store_pool.clone();
        let key = key.to_string();

        tokio::spawn(async move {
            if let Err(e) = operation(memory_pool, key.clone()).await {
                log::error!("Failed to update memory store for key {key}: {e}");
            }
        });
    }

    pub async fn new() -> Self {
        let redis_pool = init_redis().await;
        let memory_store_pool = init_memorystore().await;
        Self {
            redis_pool,
            memory_store_pool,
        }
    }

    #[deprecated(since = "0.2.0", note = "Use add_user_watch_history_items_v2 instead")]
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

    #[deprecated(
        since = "0.2.0",
        note = "Use add_user_success_history_items_v2 instead"
    )]
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

    #[deprecated(since = "0.2.0", note = "Use get_history_items_v2 instead")]
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

    #[deprecated(since = "0.2.0", note = "Use add_user_history_plain_items_v2 instead")]
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

    #[deprecated(
        since = "0.2.0",
        note = "Use is_user_history_plain_item_exists_v2 instead"
    )]
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

    #[deprecated(since = "0.2.0", note = "Use add_user_cache_items_v2 instead")]
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

    #[deprecated(since = "0.2.0", note = "Use add_global_cache_items_v2 instead")]
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

    #[deprecated(since = "0.2.0", note = "Use get_cache_items_v2 instead")]
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

    #[deprecated(since = "0.2.0", note = "Use add_user_buffer_items_v2 instead")]
    #[allow(deprecated)]
    pub async fn add_user_buffer_items(&self, items: Vec<BufferItem>) -> Result<(), anyhow::Error> {
        self.add_user_buffer_items_impl(USER_HOTORNOT_BUFFER_KEY, items)
            .await
    }

    #[deprecated(since = "0.2.0", note = "Use v2 buffer methods instead")]
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

    #[deprecated(
        since = "0.2.0",
        note = "Use get_user_buffer_items_by_timestamp_v2 instead"
    )]
    #[allow(deprecated)]
    pub async fn get_user_buffer_items_by_timestamp(
        &self,
        timestamp: u64,
    ) -> Result<Vec<BufferItem>, anyhow::Error> {
        self.get_user_buffer_items_by_timestamp_impl(USER_HOTORNOT_BUFFER_KEY, timestamp)
            .await
    }

    #[deprecated(
        since = "0.2.0",
        note = "Use get_user_buffer_items_by_timestamp_impl_v2 instead"
    )]
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

    #[deprecated(
        since = "0.2.0",
        note = "Use remove_user_buffer_items_by_timestamp_v2 instead"
    )]
    #[allow(deprecated)]
    pub async fn remove_user_buffer_items_by_timestamp(
        &self,
        timestamp_secs: u64,
    ) -> Result<u64, anyhow::Error> {
        self.remove_user_buffer_items_by_timestamp_impl(USER_HOTORNOT_BUFFER_KEY, timestamp_secs)
            .await
    }

    #[deprecated(
        since = "0.2.0",
        note = "Use remove_user_buffer_items_by_timestamp_impl_v2 instead"
    )]
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

    #[deprecated(since = "0.2.0", note = "Use delete_user_caches_v2 instead")]
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
            .map(|suffix| format!("{key}{suffix}"))
            .collect();

        // Delete all keys in one statement
        conn.del::<Vec<String>, ()>(keys).await?;

        Ok(())
    }

    // V2 Methods
    #[deprecated(since = "0.3.0", note = "Use add_user_watch_history_items_v3 instead")]
    pub async fn add_user_watch_history_items_v2(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV2>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| (get_history_item_score_v2(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV2, ()>(key, chunk)
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

        // Update memory store pool asynchronously
        let items_clone = items.clone();
        self.spawn_memory_store_update(key, move |pool, key| {
            Box::pin(async move {
                let mut conn = match pool.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        log::error!("Failed to get memory store connection: {e}");
                        return Ok(());
                    }
                };

                for chunk in items_clone.chunks(chunk_size) {
                    if let Err(e) = conn
                        .zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV2, ()>(&key, chunk)
                        .await
                    {
                        log::error!("Failed to add items to memory store: {e}");
                    }
                }

                match conn.zcard::<&str, u64>(&key).await {
                    Ok(num_items) if num_items > MAX_WATCH_HISTORY_CACHE_LEN => {
                        if let Err(e) = conn
                            .zremrangebyrank::<&str, ()>(
                                &key,
                                0,
                                (num_items - (MAX_WATCH_HISTORY_CACHE_LEN + 1)) as isize,
                            )
                            .await
                        {
                            log::error!("Failed to trim memory store: {e}");
                        }
                    }
                    Err(e) => log::error!("Failed to get card count from memory store: {e}"),
                    _ => {}
                }
                Ok(())
            })
        });

        Ok(())
    }

    #[deprecated(
        since = "0.3.0",
        note = "Use add_user_success_history_items_v3 instead"
    )]
    pub async fn add_user_success_history_items_v2(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV2>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| (get_history_item_score_v2(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV2, ()>(key, chunk)
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

        // Update memory store pool asynchronously
        let items_clone = items.clone();
        self.spawn_memory_store_update(key, move |pool, key| {
            Box::pin(async move {
                let mut conn = match pool.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        log::error!("Failed to get memory store connection: {e}");
                        return Ok(());
                    }
                };

                for chunk in items_clone.chunks(chunk_size) {
                    if let Err(e) = conn
                        .zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV2, ()>(&key, chunk)
                        .await
                    {
                        log::error!("Failed to add items to memory store: {e}");
                    }
                }

                match conn.zcard::<&str, u64>(&key).await {
                    Ok(num_items) if num_items > MAX_SUCCESS_HISTORY_CACHE_LEN => {
                        if let Err(e) = conn
                            .zremrangebyrank::<&str, ()>(
                                &key,
                                0,
                                (num_items - (MAX_SUCCESS_HISTORY_CACHE_LEN + 1)) as isize,
                            )
                            .await
                        {
                            log::error!("Failed to trim memory store: {e}");
                        }
                    }
                    Err(e) => log::error!("Failed to get card count from memory store: {e}"),
                    _ => {}
                }
                Ok(())
            })
        });

        Ok(())
    }

    #[deprecated(since = "0.3.0", note = "Use get_history_items_v3 instead")]
    pub async fn get_history_items_v2(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<MLFeedCacheHistoryItemV2>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrevrange::<&str, Vec<MLFeedCacheHistoryItemV2>>(key, start as isize, end as isize)
            .await?;

        Ok(items)
    }

    #[deprecated(since = "0.3.0", note = "Use add_user_history_plain_items_v3 instead")]
    pub async fn add_user_history_plain_items_v2(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV2>,
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
                    PlainPostItemV2 {
                        video_id: item.video_id.clone(),
                    },
                )
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, u64, PlainPostItemV2, ()>(key, chunk)
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

    #[deprecated(
        since = "0.3.0",
        note = "Use is_user_history_plain_item_exists_v3 instead"
    )]
    pub async fn is_user_history_plain_item_exists_v2(
        &self,
        key: &str,
        item: PlainPostItemV2,
    ) -> Result<bool, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let res = conn
            .zscore::<&str, PlainPostItemV2, Option<f64>>(key, item)
            .await?;

        Ok(res.is_some())
    }

    #[deprecated(since = "0.3.0", note = "Use add_user_cache_items_v3 instead")]
    pub async fn add_user_cache_items_v2(
        &self,
        key: &str,
        items: Vec<PostItemV2>,
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
            conn.zadd_multiple::<&str, f64, PostItemV2, ()>(key, chunk)
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

    #[deprecated(since = "0.3.0", note = "Use add_global_cache_items_v3 instead")]
    pub async fn add_global_cache_items_v2(
        &self,
        key: &str,
        items: Vec<PostItemV2>,
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
            conn.zadd_multiple::<&str, f64, PostItemV2, ()>(key, chunk)
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

    #[deprecated(since = "0.3.0", note = "Use get_cache_items_v3 instead")]
    pub async fn get_cache_items_v2(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<PostItemV2>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrevrange::<&str, Vec<PostItemV2>>(key, start as isize, end as isize)
            .await?;

        Ok(items)
    }

    #[deprecated(since = "0.3.0", note = "Use add_user_buffer_items_v3 instead")]
    pub async fn add_user_buffer_items_v2(
        &self,
        items: Vec<BufferItemV2>,
    ) -> Result<(), anyhow::Error> {
        self.add_user_buffer_items_impl_v2(USER_HOTORNOT_BUFFER_KEY_V2, items)
            .await
    }

    pub async fn add_user_buffer_items_impl_v2(
        &self,
        key: &str,
        items: Vec<BufferItemV2>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| {
                let timestamp_secs =
                    item.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs() as f64;
                (timestamp_secs, item.clone())
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, BufferItemV2, ()>(key, chunk)
                .await?;
        }

        Ok(())
    }

    #[deprecated(
        since = "0.3.0",
        note = "Use get_user_buffer_items_by_timestamp_v3 instead"
    )]
    pub async fn get_user_buffer_items_by_timestamp_v2(
        &self,
        timestamp_secs: u64,
    ) -> Result<Vec<BufferItemV2>, anyhow::Error> {
        self.get_user_buffer_items_by_timestamp_impl_v2(USER_HOTORNOT_BUFFER_KEY_V2, timestamp_secs)
            .await
    }

    pub async fn get_user_buffer_items_by_timestamp_impl_v2(
        &self,
        key: &str,
        timestamp_secs: u64,
    ) -> Result<Vec<BufferItemV2>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = conn
            .zrangebyscore::<&str, u64, u64, Vec<BufferItemV2>>(key, 0, timestamp_secs)
            .await?;

        Ok(items)
    }

    #[deprecated(
        since = "0.3.0",
        note = "Use remove_user_buffer_items_by_timestamp_v3 instead"
    )]
    pub async fn remove_user_buffer_items_by_timestamp_v2(
        &self,
        timestamp_secs: u64,
    ) -> Result<u64, anyhow::Error> {
        self.remove_user_buffer_items_by_timestamp_impl_v2(
            USER_HOTORNOT_BUFFER_KEY_V2,
            timestamp_secs,
        )
        .await
    }

    pub async fn remove_user_buffer_items_by_timestamp_impl_v2(
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

    #[deprecated(since = "0.3.0", note = "Use delete_user_caches_v3 instead")]
    pub async fn delete_user_caches_v2(&self, key: &str) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // All user cache suffixes for v2
        #[allow(clippy::useless_vec)]
        let suffixes = vec![
            consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_CACHE_CLEAN_SUFFIX_V2,
            consts::USER_CACHE_NSFW_SUFFIX_V2,
            consts::USER_CACHE_MIXED_SUFFIX_V2,
        ];

        // Build all keys with suffixes
        let keys: Vec<String> = suffixes
            .iter()
            .map(|suffix| format!("{key}{suffix}"))
            .collect();

        // Delete all keys in one statement
        conn.del::<Vec<String>, ()>(keys.clone()).await?;

        // Update memory store pool asynchronously
        self.spawn_memory_store_update(key, move |pool, _| {
            Box::pin(async move {
                let mut conn = match pool.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        log::error!("Failed to get memory store connection: {e}");
                        return Ok(());
                    }
                };

                if let Err(e) = conn.del::<Vec<String>, ()>(keys).await {
                    log::error!("Failed to delete keys from memory store: {e}");
                }
                Ok(())
            })
        });

        Ok(())
    }

    // V3 API Methods with String post_id

    pub async fn add_user_watch_history_items_v3(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut memory_conn = self.memory_store_pool.get().await.unwrap();

        // Extract video IDs for the watched set
        let video_ids: Vec<String> = items.iter().map(|item| item.video_id.clone()).collect();

        let items = items
            .iter()
            .map(|item| (get_history_item_score_v3(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000 to memory store first
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            memory_conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim memory store to max length
        let num_items = memory_conn.zcard::<&str, u64>(key).await?;
        if num_items > MAX_WATCH_HISTORY_CACHE_LEN {
            memory_conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_WATCH_HISTORY_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        // Add video IDs to watched set for O(1) filtering
        if !video_ids.is_empty() {
            // Extract user_id by splitting at last underscore
            let user_id = key.rsplit_once('_').map(|(prefix, _)| prefix).unwrap_or(key);
            
            let set_key = if key.contains("_nsfw") {
                format!("{}{}", user_id, consts::USER_WATCHED_VIDEO_IDS_SET_NSFW_SUFFIX_V2)
            } else {
                format!("{}{}", user_id, consts::USER_WATCHED_VIDEO_IDS_SET_CLEAN_SUFFIX_V2)
            };
            
            self.add_watched_video_ids_to_set(&set_key, video_ids).await?;
        }

        // Update persistent Redis (Upstash) in background
        let redis_pool = self.redis_pool.clone();
        let key_clone = key.to_string();
        let items_clone = items.clone();
        tokio::spawn(async move {
            match redis_pool.get().await {
                Ok(mut conn) => {
                    for chunk in items_clone.chunks(chunk_size) {
                        if let Err(e) = conn
                            .zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV3, ()>(&key_clone, chunk)
                            .await
                        {
                            log::error!("Failed to add items to persistent Redis: {e}");
                        }
                    }

                    // Trim persistent store
                    match conn.zcard::<&str, u64>(&key_clone).await {
                        Ok(num_items) if num_items > MAX_WATCH_HISTORY_CACHE_LEN => {
                            if let Err(e) = conn
                                .zremrangebyrank::<&str, ()>(
                                    &key_clone,
                                    0,
                                    (num_items - (MAX_WATCH_HISTORY_CACHE_LEN + 1)) as isize,
                                )
                                .await
                            {
                                log::error!("Failed to trim persistent Redis: {e}");
                            }
                        }
                        Err(e) => log::error!("Failed to get card count from persistent Redis: {e}"),
                        _ => {}
                    }
                }
                Err(e) => {
                    log::error!("Failed to get persistent Redis connection: {e}");
                }
            }
        });

        Ok(())
    }

    pub async fn add_user_success_history_items_v3(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut memory_conn = self.memory_store_pool.get().await.unwrap();

        // Extract video IDs for the watched set
        let video_ids: Vec<String> = items.iter().map(|item| item.video_id.clone()).collect();

        let items = items
            .iter()
            .map(|item| (get_history_item_score_v3(item), item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000 to memory store first
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            memory_conn.zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim memory store to max length
        let num_items = memory_conn.zcard::<&str, u64>(key).await?;
        if num_items > MAX_SUCCESS_HISTORY_CACHE_LEN {
            memory_conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_SUCCESS_HISTORY_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        // Add video IDs to watched set for O(1) filtering
        if !video_ids.is_empty() {
            // Extract user_id by splitting at last underscore
            let user_id = key.rsplit_once('_').map(|(prefix, _)| prefix).unwrap_or(key);
            
            let set_key = if key.contains("_nsfw") {
                format!("{}{}", user_id, consts::USER_WATCHED_VIDEO_IDS_SET_NSFW_SUFFIX_V2)
            } else {
                format!("{}{}", user_id, consts::USER_WATCHED_VIDEO_IDS_SET_CLEAN_SUFFIX_V2)
            };
            
            self.add_watched_video_ids_to_set(&set_key, video_ids).await?;
        }

        // Update persistent Redis (Upstash) in background
        let redis_pool = self.redis_pool.clone();
        let key_clone = key.to_string();
        let items_clone = items.clone();
        tokio::spawn(async move {
            match redis_pool.get().await {
                Ok(mut conn) => {
                    for chunk in items_clone.chunks(chunk_size) {
                        if let Err(e) = conn
                            .zadd_multiple::<&str, f64, MLFeedCacheHistoryItemV3, ()>(&key_clone, chunk)
                            .await
                        {
                            log::error!("Failed to add items to persistent Redis: {e}");
                        }
                    }

                    // Trim persistent store
                    match conn.zcard::<&str, u64>(&key_clone).await {
                        Ok(num_items) if num_items > MAX_SUCCESS_HISTORY_CACHE_LEN => {
                            if let Err(e) = conn
                                .zremrangebyrank::<&str, ()>(
                                    &key_clone,
                                    0,
                                    (num_items - (MAX_SUCCESS_HISTORY_CACHE_LEN + 1)) as isize,
                                )
                                .await
                            {
                                log::error!("Failed to trim persistent Redis: {e}");
                            }
                        }
                        Err(e) => log::error!("Failed to get card count from persistent Redis: {e}"),
                        _ => {}
                    }
                }
                Err(e) => {
                    log::error!("Failed to get persistent Redis connection: {e}");
                }
            }
        });

        Ok(())
    }

    pub async fn get_history_items_v3(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<MLFeedCacheHistoryItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get values from Redis - direct deserialization handles mixed types via string_or_number
        let values: Vec<MLFeedCacheHistoryItemV3> =
            conn.zrevrange(key, start as isize, end as isize).await?;

        Ok(values)
    }

    pub async fn add_user_cache_items_v3(
        &self,
        key: &str,
        items: Vec<PostItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let items = items
            .iter()
            .enumerate()
            .map(|(i, item)| (timestamp_secs + i as f64, item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, PostItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim to max length
        let num_items = conn.zcard::<&str, u64>(key).await?;
        if num_items > MAX_USER_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(key, 0, (num_items - MAX_USER_CACHE_LEN - 1) as isize)
                .await?;
        }

        Ok(())
    }

    pub async fn add_global_cache_items_v3(
        &self,
        key: &str,
        items: Vec<PostItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let items = items
            .iter()
            .enumerate()
            .map(|(i, item)| (timestamp_secs + i as f64, item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, PostItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim to max length
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

    pub async fn get_cache_items_v3(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<PostItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get raw values from Redis to handle mixed u64/String post_ids
        // Get values from Redis - direct deserialization handles mixed types via string_or_number
        let values: Vec<PostItemV3> = conn.zrevrange(key, start as isize, end as isize).await?;

        Ok(values)
    }

    pub async fn add_user_history_plain_items_v3(
        &self,
        key: &str,
        items: Vec<MLFeedCacheHistoryItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut memory_conn = self.memory_store_pool.get().await.unwrap();

        // Extract video IDs for the watched set
        let video_ids: Vec<String> = items.iter().map(|item| item.video_id.clone()).collect();

        let items = items
            .iter()
            .map(|item| {
                (
                    item.timestamp
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    PlainPostItemV3 {
                        video_id: item.video_id.clone(),
                    },
                )
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000 to memory store first
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            memory_conn.zadd_multiple::<&str, u64, PlainPostItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim memory store to max length
        let num_items = memory_conn.zcard::<&str, u64>(key).await?;
        if num_items > MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN {
            memory_conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - (MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN + 1)) as isize,
            )
            .await?;
        }

        // Add video IDs to watched set for O(1) filtering
        if !video_ids.is_empty() {
            // Extract user_id by splitting at last underscore
            let user_id = key.rsplit_once('_').map(|(prefix, _)| prefix).unwrap_or(key);
            
            // Plain post items are always clean (no nsfw variant)
            let set_key = format!("{}{}", user_id, consts::USER_WATCHED_VIDEO_IDS_SET_CLEAN_SUFFIX_V2);
            
            self.add_watched_video_ids_to_set(&set_key, video_ids).await?;
        }

        // Update persistent Redis (Upstash) in background
        let redis_pool = self.redis_pool.clone();
        let key_clone = key.to_string();
        let items_clone = items.clone();
        tokio::spawn(async move {
            match redis_pool.get().await {
                Ok(mut conn) => {
                    for chunk in items_clone.chunks(chunk_size) {
                        if let Err(e) = conn
                            .zadd_multiple::<&str, u64, PlainPostItemV3, ()>(&key_clone, chunk)
                            .await
                        {
                            log::error!("Failed to add items to persistent Redis: {e}");
                        }
                    }

                    // Trim persistent store
                    match conn.zcard::<&str, u64>(&key_clone).await {
                        Ok(num_items) if num_items > MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN => {
                            if let Err(e) = conn
                                .zremrangebyrank::<&str, ()>(
                                    &key_clone,
                                    0,
                                    (num_items - (MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN + 1)) as isize,
                                )
                                .await
                            {
                                log::error!("Failed to trim persistent Redis: {e}");
                            }
                        }
                        Err(e) => log::error!("Failed to get card count from persistent Redis: {e}"),
                        _ => {}
                    }
                }
                Err(e) => {
                    log::error!("Failed to get persistent Redis connection: {e}");
                }
            }
        });

        Ok(())
    }

    pub async fn is_user_history_plain_item_exists_v3(
        &self,
        key: &str,
        item: PlainPostItemV3,
    ) -> Result<bool, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // First try direct zscore with V3 item
        let res = conn
            .zscore::<&str, PlainPostItemV3, Option<f64>>(key, item.clone())
            .await?;

        if res.is_some() {
            return Ok(true);
        }

        // Try with V2 format (both V2 and V3 have same structure - just video_id)
        let v2_item = PlainPostItemV2 {
            video_id: item.video_id,
        };

        let res = conn
            .zscore::<&str, PlainPostItemV2, Option<f64>>(key, v2_item)
            .await?;

        Ok(res.is_some())
    }

    pub async fn add_user_buffer_items_v3(
        &self,
        items: Vec<BufferItemV3>,
    ) -> Result<(), anyhow::Error> {
        self.add_user_buffer_items_impl_v3(consts::USER_HOTORNOT_BUFFER_KEY_V3, items)
            .await
    }

    pub async fn add_user_buffer_items_impl_v3(
        &self,
        key: &str,
        items: Vec<BufferItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let items = items
            .iter()
            .map(|item| {
                let timestamp_secs =
                    item.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs() as f64;
                (timestamp_secs, item.clone())
            })
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, BufferItemV3, ()>(key, chunk)
                .await?;
        }

        Ok(())
    }

    pub async fn add_success_history_plain_post_items_v3(
        &self,
        key: &str,
        items: Vec<PlainPostItemV3>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let items = items
            .iter()
            .enumerate()
            .map(|(i, item)| (timestamp_secs + i as f64, item.clone()))
            .collect::<Vec<_>>();

        // zadd_multiple in groups of 1000
        let chunk_size = 1000;
        for chunk in items.chunks(chunk_size) {
            conn.zadd_multiple::<&str, f64, PlainPostItemV3, ()>(key, chunk)
                .await?;
        }

        // Trim to max length
        let num_items = conn.zcard::<&str, u64>(key).await?;
        if num_items > MAX_SUCCESS_HISTORY_CACHE_LEN {
            conn.zremrangebyrank::<&str, ()>(
                key,
                0,
                (num_items - MAX_SUCCESS_HISTORY_CACHE_LEN - 1) as isize,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn get_plain_post_items_v3(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<PlainPostItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get values from Redis - direct deserialization handles mixed types
        let values: Vec<PlainPostItemV3> =
            conn.zrevrange(key, start as isize, end as isize).await?;

        println!(
            "Fetched {} values from Redis for key '{}'",
            values.len(),
            key
        );

        Ok(values)
    }

    pub async fn get_user_buffer_items_by_timestamp_v3(
        &self,
        timestamp_secs: u64,
    ) -> Result<Vec<BufferItemV3>, anyhow::Error> {
        self.get_user_buffer_items_by_timestamp_impl_v3(
            consts::USER_HOTORNOT_BUFFER_KEY_V3,
            timestamp_secs,
        )
        .await
    }

    async fn get_user_buffer_items_by_timestamp_impl_v3(
        &self,
        key: &str,
        timestamp_secs: u64,
    ) -> Result<Vec<BufferItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get values from Redis - direct deserialization handles mixed types via string_or_number
        let values: Vec<BufferItemV3> = conn.zrangebyscore(key, 0, timestamp_secs).await?;

        Ok(values)
    }

    pub async fn remove_user_buffer_items_by_timestamp_v3(
        &self,
        timestamp_secs: u64,
    ) -> Result<u64, anyhow::Error> {
        self.remove_user_buffer_items_by_timestamp_impl_v3(
            consts::USER_HOTORNOT_BUFFER_KEY_V3,
            timestamp_secs,
        )
        .await
    }

    async fn remove_user_buffer_items_by_timestamp_impl_v3(
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

    pub async fn delete_user_caches_v3(&self, key: &str) -> Result<(), anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Use V2 suffixes - V3 functions operate on the same keys
        const SUFFIXES: &[&str] = &[
            consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_CACHE_CLEAN_SUFFIX_V2,
            consts::USER_CACHE_NSFW_SUFFIX_V2,
            consts::USER_CACHE_MIXED_SUFFIX_V2,
        ];

        // Build all keys with suffixes
        let keys: Vec<String> = SUFFIXES
            .iter()
            .map(|suffix| format!("{key}{suffix}"))
            .collect();

        // Delete all keys in one statement
        conn.del::<Vec<String>, ()>(keys.clone()).await?;

        // Update memory store pool asynchronously
        self.spawn_memory_store_update(key, move |pool, _| {
            Box::pin(async move {
                let mut conn = match pool.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        log::error!("Failed to get memory store connection: {e}");
                        return Ok(());
                    }
                };

                if let Err(e) = conn.del::<Vec<String>, ()>(keys).await {
                    log::error!("Failed to delete keys from memory store: {e}");
                }
                Ok(())
            })
        });

        Ok(())
    }

    // Backward compatibility helper functions

    /// Convert V2 PostItem to V3 PostItem
    pub fn convert_post_item_v2_to_v3(item: &PostItemV2) -> PostItemV3 {
        PostItemV3 {
            publisher_user_id: item.publisher_user_id.clone(),
            canister_id: item.canister_id.clone(),
            post_id: item.post_id.to_string(),
            video_id: item.video_id.clone(),
            nsfw_probability: if item.is_nsfw { 1.0 } else { 0.0 },
        }
    }

    /// Try to convert V3 PostItem to V2 PostItem (fails if post_id is not numeric)
    pub fn try_convert_post_item_v3_to_v2(item: &PostItemV3) -> Option<PostItemV2> {
        item.post_id.parse::<u64>().ok().map(|post_id| PostItemV2 {
            publisher_user_id: item.publisher_user_id.clone(),
            canister_id: item.canister_id.clone(),
            post_id,
            video_id: item.video_id.clone(),
            is_nsfw: item.is_nsfw(),
        })
    }

    /// Convert V2 history item to V3
    pub fn convert_history_item_v2_to_v3(
        item: &MLFeedCacheHistoryItemV2,
    ) -> MLFeedCacheHistoryItemV3 {
        MLFeedCacheHistoryItemV3 {
            publisher_user_id: item.publisher_user_id.clone(),
            canister_id: item.canister_id.clone(),
            post_id: item.post_id.to_string(),
            video_id: item.video_id.clone(),
            item_type: item.item_type.clone(),
            timestamp: item.timestamp,
            percent_watched: item.percent_watched,
        }
    }

    /// Try to convert V3 history item to V2 (fails if post_id is not numeric)
    pub fn try_convert_history_item_v3_to_v2(
        item: &MLFeedCacheHistoryItemV3,
    ) -> Option<MLFeedCacheHistoryItemV2> {
        item.post_id
            .parse::<u64>()
            .ok()
            .map(|post_id| MLFeedCacheHistoryItemV2 {
                publisher_user_id: item.publisher_user_id.clone(),
                canister_id: item.canister_id.clone(),
                post_id,
                video_id: item.video_id.clone(),
                item_type: item.item_type.clone(),
                timestamp: item.timestamp,
                percent_watched: item.percent_watched,
            })
    }

    /// Convert V2 buffer item to V3
    pub fn convert_buffer_item_v2_to_v3(item: &BufferItemV2) -> BufferItemV3 {
        BufferItemV3 {
            publisher_user_id: item.publisher_user_id.clone(),
            post_id: item.post_id.to_string(),
            video_id: item.video_id.clone(),
            item_type: item.item_type.clone(),
            percent_watched: item.percent_watched,
            user_id: item.user_id.clone(),
            timestamp: item.timestamp,
        }
    }

    /// Try to convert V3 buffer item to V2 (fails if post_id is not numeric)
    pub fn try_convert_buffer_item_v3_to_v2(item: &BufferItemV3) -> Option<BufferItemV2> {
        item.post_id
            .parse::<u64>()
            .ok()
            .map(|post_id| BufferItemV2 {
                publisher_user_id: item.publisher_user_id.clone(),
                post_id,
                video_id: item.video_id.clone(),
                item_type: item.item_type.clone(),
                percent_watched: item.percent_watched,
                user_id: item.user_id.clone(),
                timestamp: item.timestamp,
            })
    }

    /// Read legacy V2 data and convert to V3
    pub async fn read_legacy_as_v3(&self, key: &str) -> Result<Vec<PostItemV3>, anyhow::Error> {
        let v3_items = self.get_cache_items_v3(key, 0, u64::MAX).await?;
        Ok(v3_items)
    }

    /// Helper to parse String post_id back to u64 for legacy systems
    pub fn try_parse_legacy_post_id(post_id: &str) -> Option<u64> {
        post_id.parse().ok()
    }

    // Resilient read methods that handle mixed u64/String post_ids in Redis

    // V3 Resilient Methods

    /// Get cache items V3 with resilience to u64 post_ids from V2 data
    /// Converts u64 post_ids to String
    pub async fn get_cache_items_v3_resilient(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<PostItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get raw values from Redis
        let values: Vec<PostItemV3> = conn.zrevrange(key, start as isize, end as isize).await?;

        Ok(values)
    }

    /// Get watch history items V3 with resilience to u64 post_ids from V2 data
    pub async fn get_watch_history_items_v3_resilient(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<MLFeedCacheHistoryItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get values from Redis - direct deserialization handles mixed types via string_or_number
        let values: Vec<MLFeedCacheHistoryItemV3> =
            conn.zrevrange(key, start as isize, end as isize).await?;

        Ok(values)
    }

    /// Get buffer items V3 with resilience to u64 post_ids from V2 data
    pub async fn get_buffer_items_v3_resilient(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<BufferItemV3>, anyhow::Error> {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Get values from Redis - direct deserialization handles mixed types via string_or_number
        let values: Vec<BufferItemV3> = conn.zrevrange(key, start as isize, end as isize).await?;

        Ok(values)
    }

    pub async fn add_watched_video_ids_to_set(
        &self,
        key: &str,
        video_ids: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        if video_ids.is_empty() {
            return Ok(());
        }

        let memory_pool = self.memory_store_pool.clone();
        let mut memory_conn = memory_pool.get().await?;
        
        for video_id in &video_ids {
            let _: () = redis::cmd("SADD")
                .arg(key)
                .arg(video_id)
                .query_async(&mut *memory_conn)
                .await?;
        }

        // Check set size and trim if needed (reuse existing constant)
        let set_size: u64 = redis::cmd("SCARD")
            .arg(key)
            .query_async(&mut *memory_conn)
            .await?;

        if set_size > MAX_WATCH_HISTORY_CACHE_LEN {
            // For sets, we need to remove random members to maintain size limit
            let to_remove = set_size - MAX_WATCH_HISTORY_CACHE_LEN;
            let _: () = redis::cmd("SPOP")
                .arg(&key)
                .arg(to_remove)
                .query_async(&mut *memory_conn)
                .await?;
        }

        // Update persistent Redis in background
        let redis_pool = self.redis_pool.clone();
        let key_clone = key.to_string();
        let video_ids_clone = video_ids.clone();
        tokio::spawn(async move {
            if let Ok(mut conn) = redis_pool.get().await {
                for video_id in video_ids_clone {
                    let _: Result<(), _> = redis::cmd("SADD")
                        .arg(&key_clone)
                        .arg(&video_id)
                        .query_async(&mut *conn)
                        .await;
                }
                
                // Trim persistent store too
                let size: Result<u64, _> = redis::cmd("SCARD")
                    .arg(&key_clone)
                    .query_async(&mut *conn)
                    .await;
                if let Ok(size) = size {
                    if size > MAX_WATCH_HISTORY_CACHE_LEN {
                        let to_remove = size - MAX_WATCH_HISTORY_CACHE_LEN;
                        let _: Result<(), _> = redis::cmd("SPOP")
                            .arg(&key_clone)
                            .arg(to_remove)
                            .query_async(&mut *conn)
                            .await;
                    }
                }
            }
        });

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
        let _res = conn.del::<&str, ()>("test_watch_history_v1").await;
        assert!(_res.is_ok());

        let _res = conn.del::<&str, ()>("test_watch_history_plain_v1").await;
        assert!(_res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_watch_history_v1")
            .await
            .unwrap();
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
            .add_user_watch_history_items("test_watch_history_v1", items.clone())
            .await;
        assert!(res.is_ok());

        // add plain post items
        let res = state
            .add_user_history_plain_items("test_watch_history_plain_v1", items.clone())
            .await;
        assert!(res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_watch_history_v1")
            .await
            .unwrap();
        assert_eq!(num_items, MAX_WATCH_HISTORY_CACHE_LEN);

        let num_items_plain = conn
            .zcard::<&str, u64>("test_watch_history_plain_v1")
            .await
            .unwrap();
        assert_eq!(num_items_plain, MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN);

        let items = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItem, f64)>>(
                "test_watch_history_v1",
                0,
                4,
            )
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
                "test_watch_history_plain_v1",
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
                "test_watch_history_plain_v1",
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
        let _res = conn.del::<&str, ()>("test_success_history_v1").await;
        assert!(_res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_success_history_v1")
            .await
            .unwrap();
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
            .add_user_success_history_items("test_success_history_v1", items)
            .await;
        assert!(res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_success_history_v1")
            .await
            .unwrap();
        assert_eq!(num_items, MAX_SUCCESS_HISTORY_CACHE_LEN);

        let items = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItem, f64)>>(
                "test_success_history_v1",
                0,
                4,
            )
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

        let _res = conn.del::<&str, ()>("test_buffer_v1").await;
        assert!(_res.is_ok());

        let _res = conn.del::<&str, ()>(USER_HOTORNOT_BUFFER_KEY).await;
        assert!(_res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_buffer_v1").await.unwrap();
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
            .add_user_buffer_items_impl("test_buffer_v1", items.clone())
            .await;
        assert!(res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_buffer_v1").await.unwrap();
        assert_eq!(num_items, 100);

        let res_items = conn
            .zrevrange_withscores::<&str, Vec<(BufferItem, u64)>>("test_buffer_v1", 0, 4)
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
            .get_user_buffer_items_by_timestamp_impl("test_buffer_v1", timestamp_secs)
            .await
            .unwrap();
        assert_eq!(items.len(), 5);

        // print the items
        for item in items.iter() {
            println!("{item:?}");
        }

        // remove the items
        let res = state
            .remove_user_buffer_items_by_timestamp_impl("test_buffer_v1", timestamp_secs)
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 5);

        let num_items = conn.zcard::<&str, u64>("test_buffer_v1").await.unwrap();
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
            let full_key = format!("{test_base_key}{suffix}");
            let exists = conn.exists::<&str, bool>(&full_key).await.unwrap();
            assert!(!exists, "Key {full_key} should not exist");
        }
    }

    // V2 Tests
    #[tokio::test]
    async fn test_add_user_watch_history_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key_v2").await;
        assert!(_res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key_v2").await.unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..MAX_WATCH_HISTORY_CACHE_LEN + 10 {
            items.push(MLFeedCacheHistoryItemV2 {
                publisher_user_id: format!("test_publisher_{i}"),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                video_id: format!("test_video_id{i}"),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now() + Duration::from_secs(i * 100_u64),
                percent_watched: i as f32 / 100.0,
            });
        }

        let res = state
            .add_user_watch_history_items_v2("test_key_v2", items.clone())
            .await;
        assert!(res.is_ok());

        let num_items = conn.zcard::<&str, u64>("test_key_v2").await.unwrap();
        assert_eq!(num_items, MAX_WATCH_HISTORY_CACHE_LEN);

        // Test get_history_items_v2
        let retrieved_items = state
            .get_history_items_v2("test_key_v2", 0, 4)
            .await
            .unwrap();
        assert_eq!(retrieved_items.len(), 5);

        // Verify the items are in descending order (newest first)
        let items_with_scores = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItemV2, f64)>>("test_key_v2", 0, 4)
            .await
            .unwrap();
        assert_eq!(items_with_scores.len(), 5);

        // print the items
        for item in items_with_scores {
            println!("V2 item: {item:?}");
        }
    }

    #[tokio::test]
    async fn test_add_user_success_history_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key_success_v2").await;
        assert!(_res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_key_success_v2")
            .await
            .unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..MAX_SUCCESS_HISTORY_CACHE_LEN + 100 {
            items.push(MLFeedCacheHistoryItemV2 {
                publisher_user_id: format!("test_publisher_{i}"),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                video_id: format!("test_video_id{i}"),
                item_type: "like_video".to_string(),
                timestamp: SystemTime::now() + Duration::from_secs(i * 100_u64),
                percent_watched: 100.0,
            });
        }

        let res = state
            .add_user_success_history_items_v2("test_key_success_v2", items)
            .await;
        assert!(res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>("test_key_success_v2")
            .await
            .unwrap();
        assert_eq!(num_items, MAX_SUCCESS_HISTORY_CACHE_LEN);

        let items = conn
            .zrevrange_withscores::<&str, Vec<(MLFeedCacheHistoryItemV2, f64)>>(
                "test_key_success_v2",
                0,
                4,
            )
            .await
            .unwrap();
        assert_eq!(items.len(), 5);

        // print the items
        for item in items {
            println!("V2 success item: {item:?}");
        }
    }

    #[tokio::test]
    async fn test_user_history_plain_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key_plain_v2").await;
        assert!(_res.is_ok());

        let mut items = Vec::new();
        for i in 0..50 {
            items.push(MLFeedCacheHistoryItemV2 {
                publisher_user_id: format!("test_publisher_{i}"),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                video_id: format!("test_video_id{i}"),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now() + Duration::from_secs(i * 10),
                percent_watched: 50.0,
            });
        }

        // Add plain items (now uses Sorted Set)
        let res = state
            .add_user_history_plain_items_v2("test_key_plain_v2", items.clone())
            .await;
        assert!(res.is_ok());

        // Verify items are stored in sorted set
        let num_items = conn.zcard::<&str, u64>("test_key_plain_v2").await.unwrap();
        assert_eq!(num_items, 50);

        // Check if specific item exists
        let exists = state
            .is_user_history_plain_item_exists_v2(
                "test_key_plain_v2",
                PlainPostItemV2 {
                    video_id: "test_video_id25".to_string(),
                },
            )
            .await
            .unwrap();
        assert!(exists);

        // Check if non-existent item
        let not_exists = state
            .is_user_history_plain_item_exists_v2(
                "test_key_plain_v2",
                PlainPostItemV2 {
                    video_id: "test_video_id999".to_string(),
                },
            )
            .await
            .unwrap();
        assert!(!not_exists);

        // Test with more items than limit
        let mut many_items = Vec::new();
        for i in 0..MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN + 100 {
            many_items.push(MLFeedCacheHistoryItemV2 {
                publisher_user_id: format!("test_publisher_{i}"),
                canister_id: "test_canister_id".to_string(),
                post_id: i,
                video_id: format!("test_video_id_many{i}"),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now() + Duration::from_secs(i * 10),
                percent_watched: 50.0,
            });
        }

        let _res = conn.del::<&str, ()>("test_key_plain_v2_many").await;
        assert!(_res.is_ok());

        let res = state
            .add_user_history_plain_items_v2("test_key_plain_v2_many", many_items)
            .await;
        assert!(res.is_ok());

        // Check that the sorted set size is limited
        let num_items = conn
            .zcard::<&str, u64>("test_key_plain_v2_many")
            .await
            .unwrap();
        assert_eq!(num_items, MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN);

        // Verify oldest items were removed by checking the scores
        let oldest_items = conn
            .zrange_withscores::<&str, Vec<(PlainPostItemV2, u64)>>("test_key_plain_v2_many", 0, 0)
            .await
            .unwrap();
        assert!(!oldest_items.is_empty());

        // The oldest item should not be from the first 100 items (they should have been removed)
        let oldest_video_id = &oldest_items[0].0.video_id;
        assert!(oldest_video_id.contains("test_video_id_many"));
        let id_num: u64 = oldest_video_id
            .replace("test_video_id_many", "")
            .parse()
            .unwrap();
        assert!(id_num >= 100); // First 100 items should have been removed
    }

    #[tokio::test]
    async fn test_user_cache_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key_cache_v2").await;
        assert!(_res.is_ok());

        let test_items = vec![
            PostItemV2 {
                publisher_user_id: "publisher1".to_string(),
                canister_id: "canister1".to_string(),
                post_id: 1,
                video_id: "video1".to_string(),
                is_nsfw: false,
            },
            PostItemV2 {
                publisher_user_id: "publisher2".to_string(),
                canister_id: "canister2".to_string(),
                post_id: 2,
                video_id: "video2".to_string(),
                is_nsfw: true,
            },
            PostItemV2 {
                publisher_user_id: "publisher3".to_string(),
                canister_id: "canister3".to_string(),
                post_id: 3,
                video_id: "video3".to_string(),
                is_nsfw: false,
            },
        ];

        // Add items (now uses Sorted Set)
        let res = state
            .add_user_cache_items_v2("test_key_cache_v2", test_items.clone())
            .await;
        assert!(res.is_ok());

        // Get items back
        let retrieved = state
            .get_cache_items_v2("test_key_cache_v2", 0, 2)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 3);

        // Since all items have the same timestamp, order might vary
        // Just check that all expected videos are present
        let video_ids: Vec<String> = retrieved.iter().map(|item| item.video_id.clone()).collect();
        assert!(video_ids.contains(&"video1".to_string()));
        assert!(video_ids.contains(&"video2".to_string()));
        assert!(video_ids.contains(&"video3".to_string()));

        // Test with more items than limit
        let mut many_items = Vec::new();
        for i in 0..MAX_USER_CACHE_LEN + 50 {
            many_items.push(PostItemV2 {
                publisher_user_id: format!("publisher_{i}"),
                canister_id: format!("canister_{i}"),
                post_id: i,
                video_id: format!("video_{i}"),
                is_nsfw: false,
            });
        }

        let res = state
            .add_user_cache_items_v2("test_key_cache_v2", many_items)
            .await;
        assert!(res.is_ok());

        // Check that the sorted set size is limited
        let num_items = conn.zcard::<&str, u64>("test_key_cache_v2").await.unwrap();
        assert_eq!(num_items, MAX_USER_CACHE_LEN);
    }

    #[tokio::test]
    async fn test_global_cache_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();
        // delete the key
        let _res = conn.del::<&str, ()>("test_key_global_v2").await;
        assert!(_res.is_ok());

        let test_items = vec![
            PostItemV2 {
                publisher_user_id: "global_publisher1".to_string(),
                canister_id: "global_canister1".to_string(),
                post_id: 1,
                video_id: "global_video1".to_string(),
                is_nsfw: false,
            },
            PostItemV2 {
                publisher_user_id: "global_publisher2".to_string(),
                canister_id: "global_canister2".to_string(),
                post_id: 2,
                video_id: "global_video2".to_string(),
                is_nsfw: true,
            },
        ];

        // Add items (now uses Sorted Set)
        let res = state
            .add_global_cache_items_v2("test_key_global_v2", test_items.clone())
            .await;
        assert!(res.is_ok());

        // Get items back
        let retrieved = state
            .get_cache_items_v2("test_key_global_v2", 0, 1)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 2);

        // Since all items have the same timestamp, order might vary
        // Just check that all expected videos are present
        let video_ids: Vec<String> = retrieved.iter().map(|item| item.video_id.clone()).collect();
        assert!(video_ids.contains(&"global_video1".to_string()));
        assert!(video_ids.contains(&"global_video2".to_string()));

        // Test with more items than limit
        let mut many_items = Vec::new();
        for i in 0..MAX_GLOBAL_CACHE_LEN + 100 {
            many_items.push(PostItemV2 {
                publisher_user_id: format!("global_publisher_{i}"),
                canister_id: format!("global_canister_{i}"),
                post_id: i,
                video_id: format!("global_video_{i}"),
                is_nsfw: false,
            });
        }

        let res = state
            .add_global_cache_items_v2("test_key_global_v2", many_items)
            .await;
        assert!(res.is_ok());

        // Check that the sorted set size is limited
        let num_items = conn.zcard::<&str, u64>("test_key_global_v2").await.unwrap();
        assert_eq!(num_items, MAX_GLOBAL_CACHE_LEN);
    }

    #[tokio::test]
    async fn test_user_buffer_items_v2() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();

        let _res = conn.del::<&str, ()>(USER_HOTORNOT_BUFFER_KEY_V2).await;
        assert!(_res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>(USER_HOTORNOT_BUFFER_KEY_V2)
            .await
            .unwrap();
        assert_eq!(num_items, 0);

        let mut items = Vec::new();
        for i in 0..100 {
            items.push(BufferItemV2 {
                publisher_user_id: format!("test_publisher_{i}"),
                post_id: i,
                video_id: format!("test_video_id{i}"),
                item_type: "video_viewed".to_string(),
                percent_watched: 50.0 + (i as f32),
                user_id: format!("test_user_{i}"),
                timestamp: SystemTime::now() + Duration::from_secs(i * 100_u64),
            });
        }

        // Add buffer items
        let res = state.add_user_buffer_items_v2(items.clone()).await;
        assert!(res.is_ok());

        let num_items = conn
            .zcard::<&str, u64>(USER_HOTORNOT_BUFFER_KEY_V2)
            .await
            .unwrap();
        assert_eq!(num_items, 100);

        // Test get_user_buffer_items_by_timestamp_v2
        let timestamp = items[4].timestamp;
        let timestamp_secs = timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let retrieved_items = state
            .get_user_buffer_items_by_timestamp_v2(timestamp_secs)
            .await
            .unwrap();
        assert_eq!(retrieved_items.len(), 5);

        // Verify items are in ascending order by timestamp
        for i in 0..4 {
            let t1 = retrieved_items[i]
                .timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let t2 = retrieved_items[i + 1]
                .timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            assert!(t1 <= t2);
        }

        // print the items
        for item in retrieved_items.iter() {
            println!("V2 buffer item: {item:?}");
        }

        // Test remove_user_buffer_items_by_timestamp_v2
        let res = state
            .remove_user_buffer_items_by_timestamp_v2(timestamp_secs)
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 5);

        let num_items = conn
            .zcard::<&str, u64>(USER_HOTORNOT_BUFFER_KEY_V2)
            .await
            .unwrap();
        assert_eq!(num_items, 95);

        // Verify removed items are gone
        let retrieved_after_remove = state
            .get_user_buffer_items_by_timestamp_v2(timestamp_secs)
            .await
            .unwrap();
        assert_eq!(retrieved_after_remove.len(), 0);
    }

    #[tokio::test]
    async fn test_v2_buffer_impl_methods() {
        let state = MLFeedCacheState::new().await;

        let mut conn = state.redis_pool.get().await.unwrap();

        let test_key = "test_buffer_key_v2";
        let _res = conn.del::<&str, ()>(test_key).await;
        assert!(_res.is_ok());

        let mut items = Vec::new();
        for i in 0..50 {
            items.push(BufferItemV2 {
                publisher_user_id: format!("impl_publisher_{i}"),
                post_id: i,
                video_id: format!("impl_video_id{i}"),
                item_type: "like_video".to_string(),
                percent_watched: 100.0,
                user_id: format!("impl_user_{i}"),
                timestamp: SystemTime::now() + Duration::from_secs(i * 50_u64),
            });
        }

        // Test get_user_buffer_items_by_timestamp_impl_v2
        let timestamp = items[9].timestamp;
        let timestamp_secs = timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Add items to custom key using internal method (can't directly access it, so we'll add via Redis)
        let items_with_scores: Vec<(f64, BufferItemV2)> = items
            .iter()
            .map(|item| {
                let ts = item.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs() as f64;
                (ts, item.clone())
            })
            .collect();

        // Add in chunks
        for chunk in items_with_scores.chunks(10) {
            conn.zadd_multiple::<&str, f64, BufferItemV2, ()>(test_key, chunk)
                .await
                .unwrap();
        }

        let retrieved = state
            .get_user_buffer_items_by_timestamp_impl_v2(test_key, timestamp_secs)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 10);

        // Test remove_user_buffer_items_by_timestamp_impl_v2
        let removed = state
            .remove_user_buffer_items_by_timestamp_impl_v2(test_key, timestamp_secs)
            .await
            .unwrap();
        assert_eq!(removed, 10);

        let remaining = conn.zcard::<&str, u64>(test_key).await.unwrap();
        assert_eq!(remaining, 40);
    }

    #[tokio::test]
    async fn test_delete_user_caches_v2() {
        let state = MLFeedCacheState::new().await;
        let mut conn = state.redis_pool.get().await.unwrap();

        let test_base_key = "test_user_delete_v2";

        // Create some test data for each v2 cache type
        let test_items = vec![
            PostItemV2 {
                publisher_user_id: "test_publisher".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 1,
                video_id: "test_video_1".to_string(),
                is_nsfw: false,
            },
            PostItemV2 {
                publisher_user_id: "test_publisher2".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 2,
                video_id: "test_video_2".to_string(),
                is_nsfw: true,
            },
        ];

        let history_items = vec![
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "test_publisher_1".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 1,
                video_id: "test_video_1".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "test_publisher_2".to_string(),
                canister_id: "test_canister".to_string(),
                post_id: 2,
                video_id: "test_video_2".to_string(),
                item_type: "like_video".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 100.0,
            },
        ];

        // Add data to various v2 cache types
        state
            .add_user_cache_items_v2(
                &format!("{}{}", test_base_key, consts::USER_CACHE_CLEAN_SUFFIX_V2),
                test_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_cache_items_v2(
                &format!("{}{}", test_base_key, consts::USER_CACHE_NSFW_SUFFIX_V2),
                test_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_cache_items_v2(
                &format!("{}{}", test_base_key, consts::USER_CACHE_MIXED_SUFFIX_V2),
                test_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_watch_history_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_watch_history_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2
                ),
                history_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_success_history_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_success_history_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2
                ),
                history_items.clone(),
            )
            .await
            .unwrap();

        state
            .add_user_history_plain_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2
                ),
                history_items.clone(),
            )
            .await
            .unwrap();
        state
            .add_user_history_plain_items_v2(
                &format!(
                    "{}{}",
                    test_base_key,
                    consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2
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
                consts::USER_CACHE_CLEAN_SUFFIX_V2
            ))
            .await
            .unwrap();
        assert_eq!(cache_clean_len, 2);

        let watch_clean_len = conn
            .zcard::<&str, u64>(&format!(
                "{}{}",
                test_base_key,
                consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2
            ))
            .await
            .unwrap();
        assert_eq!(watch_clean_len, 2);

        // Delete all user v2 caches
        state.delete_user_caches_v2(test_base_key).await.unwrap();

        // Verify all v2 caches are deleted
        let suffixes = vec![
            consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2,
            consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2,
            consts::USER_CACHE_CLEAN_SUFFIX_V2,
            consts::USER_CACHE_NSFW_SUFFIX_V2,
            consts::USER_CACHE_MIXED_SUFFIX_V2,
        ];

        for suffix in suffixes {
            let full_key = format!("{test_base_key}{suffix}");
            let exists = conn.exists::<&str, bool>(&full_key).await.unwrap();
            assert!(!exists, "Key {full_key} should not exist");
        }
    }

    #[tokio::test]
    async fn test_v3_types_with_string_post_id() {
        let state = MLFeedCacheState::new().await;
        let mut conn = state.redis_pool.get().await.unwrap();
        let test_key = "test_v3_types";

        // Clean up
        let _res = conn.del::<&str, ()>(test_key).await;

        // Test PostItemV3 with string post_ids
        let items = vec![
            PostItemV3 {
                publisher_user_id: "user1".to_string(),
                canister_id: "canister1".to_string(),
                post_id: "123".to_string(), // Numeric string
                video_id: "video1".to_string(),
                nsfw_probability: 1.0,
            },
            PostItemV3 {
                publisher_user_id: "user2".to_string(),
                canister_id: "canister2".to_string(),
                post_id: "abc-456-def".to_string(), // Non-numeric string
                video_id: "video2".to_string(),
                nsfw_probability: 0.0,
            },
        ];

        // Add V3 items to cache
        let res = state.add_user_cache_items_v3(test_key, items.clone()).await;
        assert!(res.is_ok());

        // Retrieve V3 items
        let retrieved = state.get_cache_items_v3(test_key, 0, 10).await.unwrap();
        assert_eq!(retrieved.len(), 2);

        // Verify post_ids are strings
        assert!(retrieved.iter().any(|item| item.post_id == "123"));
        assert!(retrieved.iter().any(|item| item.post_id == "abc-456-def"));

        // Clean up
        conn.del::<&str, ()>(test_key).await.unwrap();
    }

    #[tokio::test]
    async fn test_v3_history_items() {
        let state = MLFeedCacheState::new().await;
        let mut conn = state.redis_pool.get().await.unwrap();
        let test_key = "test_v3_history";

        // Clean up
        let _res = conn.del::<&str, ()>(test_key).await;

        // Test MLFeedCacheHistoryItemV3
        let items = vec![MLFeedCacheHistoryItemV3 {
            publisher_user_id: "pub1".to_string(),
            canister_id: "can1".to_string(),
            post_id: "post-id-string".to_string(),
            video_id: "vid1".to_string(),
            item_type: "view".to_string(),
            timestamp: SystemTime::now(),
            percent_watched: 0.75,
        }];

        let res = state.add_user_watch_history_items_v3(test_key, items).await;
        assert!(res.is_ok());

        // Clean up
        conn.del::<&str, ()>(test_key).await.unwrap();
    }

    #[test]
    fn test_v2_to_v3_conversion() {
        // Test PostItemV2 to PostItemV3 conversion
        let v2_item = PostItemV2 {
            publisher_user_id: "user1".to_string(),
            canister_id: "can1".to_string(),
            post_id: 12345u64,
            video_id: "vid1".to_string(),
            is_nsfw: false,
        };

        let v3_item = MLFeedCacheState::convert_post_item_v2_to_v3(&v2_item);
        assert_eq!(v3_item.post_id, "12345");
        assert_eq!(v3_item.publisher_user_id, v2_item.publisher_user_id);
        assert_eq!(v3_item.video_id, v2_item.video_id);

        // Test V3 to V2 conversion (should succeed for numeric strings)
        let result = MLFeedCacheState::try_convert_post_item_v3_to_v2(&v3_item);
        assert!(result.is_some());
        let v2_back = result.unwrap();
        assert_eq!(v2_back.post_id, 12345u64);

        // Test V3 to V2 conversion with non-numeric string (should fail)
        let v3_non_numeric = PostItemV3 {
            publisher_user_id: "user1".to_string(),
            canister_id: "can1".to_string(),
            post_id: "not-a-number".to_string(),
            video_id: "vid1".to_string(),
            nsfw_probability: 0.0,
        };

        let result = MLFeedCacheState::try_convert_post_item_v3_to_v2(&v3_non_numeric);
        assert!(result.is_none());
    }

    #[test]
    fn test_history_item_conversions() {
        let v2_item = MLFeedCacheHistoryItemV2 {
            publisher_user_id: "pub1".to_string(),
            canister_id: "can1".to_string(),
            post_id: 999u64,
            video_id: "vid1".to_string(),
            item_type: "view".to_string(),
            timestamp: SystemTime::now(),
            percent_watched: 0.5,
        };

        let v3_item = MLFeedCacheState::convert_history_item_v2_to_v3(&v2_item);
        assert_eq!(v3_item.post_id, "999");

        let v2_back = MLFeedCacheState::try_convert_history_item_v3_to_v2(&v3_item);
        assert!(v2_back.is_some());
        assert_eq!(v2_back.unwrap().post_id, 999u64);
    }

    #[test]
    fn test_buffer_item_conversions() {
        let v2_item = BufferItemV2 {
            publisher_user_id: "pub1".to_string(),
            post_id: 777u64,
            video_id: "vid1".to_string(),
            item_type: "like".to_string(),
            percent_watched: 1.0,
            user_id: "user1".to_string(),
            timestamp: SystemTime::now(),
        };

        let v3_item = MLFeedCacheState::convert_buffer_item_v2_to_v3(&v2_item);
        assert_eq!(v3_item.post_id, "777");

        let v2_back = MLFeedCacheState::try_convert_buffer_item_v3_to_v2(&v3_item);
        assert!(v2_back.is_some());
        assert_eq!(v2_back.unwrap().post_id, 777u64);
    }

    #[test]
    fn test_legacy_post_id_parsing() {
        // Test valid numeric strings
        assert_eq!(
            MLFeedCacheState::try_parse_legacy_post_id("123"),
            Some(123u64)
        );
        assert_eq!(MLFeedCacheState::try_parse_legacy_post_id("0"), Some(0u64));
        assert_eq!(
            MLFeedCacheState::try_parse_legacy_post_id("999999"),
            Some(999999u64)
        );

        // Test invalid strings
        assert_eq!(MLFeedCacheState::try_parse_legacy_post_id("abc"), None);
        assert_eq!(MLFeedCacheState::try_parse_legacy_post_id("123-456"), None);
        assert_eq!(MLFeedCacheState::try_parse_legacy_post_id(""), None);
        assert_eq!(MLFeedCacheState::try_parse_legacy_post_id("12.34"), None);
    }

    #[tokio::test]
    async fn test_v2_v3_mixed_data_resilient_methods() {
        let state = MLFeedCacheState::new().await;

        // Clean up test keys
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>("test_mixed_history").await;
        let _ = conn.del::<&str, ()>("test_mixed_cache").await;
        let _ = conn.del::<&str, ()>("test_mixed_buffer").await;

        // Test 1: Mixed history items (V2 with u64 post_id and V3 with String post_id)
        {
            let key = "test_mixed_history";

            // Add V2 items with u64 post_ids
            let v2_items = vec![
                MLFeedCacheHistoryItemV2 {
                    publisher_user_id: "publisher1".to_string(),
                    canister_id: "canister1".to_string(),
                    post_id: 12345,
                    video_id: "video1".to_string(),
                    item_type: "video_viewed".to_string(),
                    timestamp: SystemTime::now(),
                    percent_watched: 75.0,
                },
                MLFeedCacheHistoryItemV2 {
                    publisher_user_id: "publisher2".to_string(),
                    canister_id: "canister2".to_string(),
                    post_id: 67890,
                    video_id: "video2".to_string(),
                    item_type: "like_video".to_string(),
                    timestamp: SystemTime::now(),
                    percent_watched: 100.0,
                },
            ];

            state
                .add_user_watch_history_items_v2(key, v2_items.clone())
                .await
                .unwrap();

            // Add V3 items with String post_ids
            let v3_items = vec![
                MLFeedCacheHistoryItemV3 {
                    publisher_user_id: "publisher3".to_string(),
                    canister_id: "canister3".to_string(),
                    post_id: "abc123def".to_string(), // Non-numeric String
                    video_id: "video3".to_string(),
                    item_type: "video_viewed".to_string(),
                    timestamp: SystemTime::now(),
                    percent_watched: 50.0,
                },
                MLFeedCacheHistoryItemV3 {
                    publisher_user_id: "publisher4".to_string(),
                    canister_id: "canister4".to_string(),
                    post_id: "999999".to_string(), // Numeric String
                    video_id: "video4".to_string(),
                    item_type: "video_viewed".to_string(),
                    timestamp: SystemTime::now(),
                    percent_watched: 25.0,
                },
            ];

            state
                .add_user_watch_history_items_v3(key, v3_items.clone())
                .await
                .unwrap();

            // Read using V3 resilient method - should get all items with post_ids converted to String
            let retrieved_items = state
                .get_watch_history_items_v3_resilient(key, 0, 10)
                .await
                .unwrap();

            assert_eq!(retrieved_items.len(), 4, "Should retrieve all 4 items");

            // Verify V2 items are converted correctly (post_id u64 -> String)
            let v2_converted: Vec<_> = retrieved_items
                .iter()
                .filter(|item| item.video_id == "video1" || item.video_id == "video2")
                .collect();
            assert_eq!(v2_converted.len(), 2);

            // Check that u64 post_ids were converted to String
            for item in &v2_converted {
                if item.video_id == "video1" {
                    assert_eq!(item.post_id, "12345");
                } else if item.video_id == "video2" {
                    assert_eq!(item.post_id, "67890");
                }
            }

            // Verify V3 items remain unchanged
            let v3_items: Vec<_> = retrieved_items
                .iter()
                .filter(|item| item.video_id == "video3" || item.video_id == "video4")
                .collect();
            assert_eq!(v3_items.len(), 2);

            for item in &v3_items {
                if item.video_id == "video3" {
                    assert_eq!(item.post_id, "abc123def");
                } else if item.video_id == "video4" {
                    assert_eq!(item.post_id, "999999");
                }
            }
        }

        // Test 2: Mixed cache items (PostItemV2 and PostItemV3)
        {
            let key = "test_mixed_cache";

            // Add V2 cache items with u64 post_ids
            let v2_items = vec![
                PostItemV2 {
                    publisher_user_id: "pub1".to_string(),
                    canister_id: "can1".to_string(),
                    post_id: 11111,
                    video_id: "cache_video1".to_string(),
                    is_nsfw: false,
                },
                PostItemV2 {
                    publisher_user_id: "pub2".to_string(),
                    canister_id: "can2".to_string(),
                    post_id: 22222,
                    video_id: "cache_video2".to_string(),
                    is_nsfw: true,
                },
            ];

            state.add_user_cache_items_v2(key, v2_items).await.unwrap();

            // Add V3 cache items with String post_ids
            let v3_items = vec![
                PostItemV3 {
                    publisher_user_id: "pub3".to_string(),
                    canister_id: "can3".to_string(),
                    post_id: "xyz789".to_string(),
                    video_id: "cache_video3".to_string(),
                    nsfw_probability: 0.0,
                },
                PostItemV3 {
                    publisher_user_id: "pub4".to_string(),
                    canister_id: "can4".to_string(),
                    post_id: "33333".to_string(),
                    video_id: "cache_video4".to_string(),
                    nsfw_probability: 0.0,
                },
            ];

            state.add_user_cache_items_v3(key, v3_items).await.unwrap();

            // Read using V3 resilient method
            let retrieved_items = state
                .get_cache_items_v3_resilient(key, 0, 10)
                .await
                .unwrap();

            assert_eq!(
                retrieved_items.len(),
                4,
                "Should retrieve all 4 cache items"
            );

            // Verify conversions
            for item in &retrieved_items {
                match item.video_id.as_str() {
                    "cache_video1" => {
                        assert_eq!(item.post_id, "11111");
                        assert!(!item.is_nsfw());
                    }
                    "cache_video2" => {
                        assert_eq!(item.post_id, "22222");
                        assert!(item.is_nsfw());
                    }
                    "cache_video3" => assert_eq!(item.post_id, "xyz789"),
                    "cache_video4" => assert_eq!(item.post_id, "33333"),
                    _ => panic!("Unexpected video_id"),
                }
            }
        }

        // Test 3: Mixed buffer items
        {
            // Add V2 buffer items with u64 post_ids
            let v2_items = vec![
                BufferItemV2 {
                    publisher_user_id: "buf_pub1".to_string(),
                    post_id: 44444,
                    video_id: "buf_video1".to_string(),
                    item_type: "video_viewed".to_string(),
                    percent_watched: 60.0,
                    user_id: "user1".to_string(),
                    timestamp: SystemTime::now(),
                },
                BufferItemV2 {
                    publisher_user_id: "buf_pub2".to_string(),
                    post_id: 55555,
                    video_id: "buf_video2".to_string(),
                    item_type: "like_video".to_string(),
                    percent_watched: 100.0,
                    user_id: "user2".to_string(),
                    timestamp: SystemTime::now(),
                },
            ];

            state.add_user_buffer_items_v2(v2_items).await.unwrap();

            // Add V3 buffer items with String post_ids
            let v3_items = vec![BufferItemV3 {
                publisher_user_id: "buf_pub3".to_string(),
                post_id: "buffer_post_abc".to_string(),
                video_id: "buf_video3".to_string(),
                item_type: "video_viewed".to_string(),
                percent_watched: 80.0,
                user_id: "user3".to_string(),
                timestamp: SystemTime::now(),
            }];

            state.add_user_buffer_items_v3(v3_items).await.unwrap();

            // Sleep briefly to ensure items are in buffer
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Read using V3 resilient method
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 1; // Add 1 second to ensure we get all items

            let retrieved_items = state
                .get_buffer_items_v3_resilient(consts::USER_HOTORNOT_BUFFER_KEY_V2, 0, 10)
                .await
                .unwrap();

            // We should get at least the V3 item, V2 items may vary based on timing
            assert!(
                !retrieved_items.is_empty(),
                "Should retrieve at least some buffer items"
            );

            // Verify any retrieved items have correct post_id format
            for item in &retrieved_items {
                match item.video_id.as_str() {
                    "buf_video1" => assert_eq!(item.post_id, "44444"),
                    "buf_video2" => assert_eq!(item.post_id, "55555"),
                    "buf_video3" => assert_eq!(item.post_id, "buffer_post_abc"),
                    _ => {} // Other items might exist from other tests
                }
            }
        }

        // Clean up
        let _ = conn.del::<&str, ()>("test_mixed_history").await;
        let _ = conn.del::<&str, ()>("test_mixed_cache").await;

        println!("✅ V2/V3 mixed data resilient methods test passed!");
    }

    #[tokio::test]
    async fn test_v3_get_history_items_resilient() {
        let state = MLFeedCacheState::new().await;

        // Clean up test key
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>("test_v3_history_resilient").await;

        let key = "test_v3_history_resilient";

        // Add V2 items with u64 post_ids
        let v2_items = vec![
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub1".to_string(),
                canister_id: "can1".to_string(),
                post_id: 10001,
                video_id: "vid1".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub2".to_string(),
                canister_id: "can2".to_string(),
                post_id: 20002,
                video_id: "vid2".to_string(),
                item_type: "like_video".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 100.0,
            },
        ];

        state
            .add_user_watch_history_items_v2(key, v2_items)
            .await
            .unwrap();

        // Add V3 items with String post_ids
        let v3_items = vec![
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub3".to_string(),
                canister_id: "can3".to_string(),
                post_id: "string_post_123".to_string(),
                video_id: "vid3".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 75.0,
            },
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub4".to_string(),
                canister_id: "can4".to_string(),
                post_id: "30003".to_string(), // Numeric string
                video_id: "vid4".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 25.0,
            },
        ];

        state
            .add_user_watch_history_items_v3(key, v3_items)
            .await
            .unwrap();

        // Read using V3 method (which now uses resilient logic internally)
        let retrieved = state.get_history_items_v3(key, 0, 10).await.unwrap();

        assert_eq!(retrieved.len(), 4, "Should retrieve all 4 items");

        // Verify u64 post_ids were converted to String
        let converted_ids: Vec<String> =
            retrieved.iter().map(|item| item.post_id.clone()).collect();
        assert!(converted_ids.contains(&"10001".to_string()));
        assert!(converted_ids.contains(&"20002".to_string()));
        assert!(converted_ids.contains(&"string_post_123".to_string()));
        assert!(converted_ids.contains(&"30003".to_string()));

        // Verify video_ids are preserved
        let video_ids: Vec<String> = retrieved.iter().map(|item| item.video_id.clone()).collect();
        assert!(video_ids.contains(&"vid1".to_string()));
        assert!(video_ids.contains(&"vid2".to_string()));
        assert!(video_ids.contains(&"vid3".to_string()));
        assert!(video_ids.contains(&"vid4".to_string()));
    }

    #[tokio::test]
    async fn test_v3_get_cache_items_resilient() {
        let state = MLFeedCacheState::new().await;

        // Clean up test key
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>("test_v3_cache_resilient").await;

        let key = "test_v3_cache_resilient";

        // Add V2 cache items with u64 post_ids
        let v2_items = vec![
            PostItemV2 {
                publisher_user_id: "user1".to_string(),
                canister_id: "can1".to_string(),
                post_id: 5001,
                video_id: "cache_vid1".to_string(),
                is_nsfw: false,
            },
            PostItemV2 {
                publisher_user_id: "user2".to_string(),
                canister_id: "can2".to_string(),
                post_id: 5002,
                video_id: "cache_vid2".to_string(),
                is_nsfw: true,
            },
        ];

        state.add_user_cache_items_v2(key, v2_items).await.unwrap();

        // Add V3 cache items with String post_ids
        let v3_items = vec![
            PostItemV3 {
                publisher_user_id: "user3".to_string(),
                canister_id: "can3".to_string(),
                post_id: "non_numeric_id".to_string(),
                video_id: "cache_vid3".to_string(),
                nsfw_probability: 0.0,
            },
            PostItemV3 {
                publisher_user_id: "user4".to_string(),
                canister_id: "can4".to_string(),
                post_id: "5003".to_string(),
                video_id: "cache_vid4".to_string(),
                nsfw_probability: 0.0,
            },
        ];

        state.add_user_cache_items_v3(key, v3_items).await.unwrap();

        // Read using V3 method (which now uses resilient logic internally)
        let retrieved = state.get_cache_items_v3(key, 0, 10).await.unwrap();

        assert_eq!(retrieved.len(), 4, "Should retrieve all 4 cache items");

        // Verify all post_ids are now Strings
        for item in &retrieved {
            match item.video_id.as_str() {
                "cache_vid1" => assert_eq!(item.post_id, "5001"),
                "cache_vid2" => {
                    assert_eq!(item.post_id, "5002");
                    assert!(item.is_nsfw());
                }
                "cache_vid3" => assert_eq!(item.post_id, "non_numeric_id"),
                "cache_vid4" => assert_eq!(item.post_id, "5003"),
                _ => panic!("Unexpected video_id"),
            }
        }
    }

    #[tokio::test]
    async fn test_v3_get_plain_post_items_resilient() {
        let state = MLFeedCacheState::new().await;

        // Clean up test key
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>("test_v3_plain_resilient").await;

        let key = "test_v3_plain_resilient";

        // Add plain post items through history items (since that's how they're typically added)
        let history_items_v2 = vec![
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub1".to_string(),
                canister_id: "can1".to_string(),
                post_id: 7001,
                video_id: "plain_vid1".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub2".to_string(),
                canister_id: "can2".to_string(),
                post_id: 7002,
                video_id: "plain_vid2".to_string(),
                item_type: "like_video".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 100.0,
            },
        ];

        state
            .add_user_history_plain_items_v2(key, history_items_v2)
            .await
            .unwrap();

        // Add V3 plain items
        let history_items_v3 = vec![
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub3".to_string(),
                canister_id: "can3".to_string(),
                post_id: "plain_string_id".to_string(),
                video_id: "plain_vid3".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 75.0,
            },
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub4".to_string(),
                canister_id: "can4".to_string(),
                post_id: "7003".to_string(),
                video_id: "plain_vid4".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: SystemTime::now(),
                percent_watched: 25.0,
            },
        ];

        state
            .add_user_history_plain_items_v3(key, history_items_v3)
            .await
            .unwrap();

        // Read using V3 method (which now uses resilient logic internally)
        let retrieved = state.get_plain_post_items_v3(key, 0, 10).await.unwrap();

        // Plain items should have unique entries based on video_id
        // V2 and V3 both create PlainPostItem with only video_id
        assert_eq!(retrieved.len(), 4, "Should retrieve exactly 4 plain items");

        // Verify video_ids are all present
        let video_ids: Vec<String> = retrieved.iter().map(|item| item.video_id.clone()).collect();

        // Check that all video_ids are present
        assert!(video_ids.contains(&"plain_vid1".to_string()));
        assert!(video_ids.contains(&"plain_vid2".to_string()));
        assert!(video_ids.contains(&"plain_vid3".to_string()));
        assert!(video_ids.contains(&"plain_vid4".to_string()));
    }

    #[tokio::test]
    async fn test_v3_get_buffer_items_by_timestamp_resilient() {
        let state = MLFeedCacheState::new().await;

        // Use test-specific key
        let test_key = "test_v3_buffer_resilient";

        // Clean up test buffer
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>(test_key).await;

        let base_time = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add V2 buffer items with u64 post_ids
        let v2_items = vec![
            BufferItemV2 {
                publisher_user_id: "buf_pub1".to_string(),
                post_id: 9001,
                video_id: "buf_vid1".to_string(),
                item_type: "video_viewed".to_string(),
                percent_watched: 50.0,
                user_id: "user1".to_string(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(base_time - 100),
            },
            BufferItemV2 {
                publisher_user_id: "buf_pub2".to_string(),
                post_id: 9002,
                video_id: "buf_vid2".to_string(),
                item_type: "like_video".to_string(),
                percent_watched: 100.0,
                user_id: "user2".to_string(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(base_time - 50),
            },
        ];

        state
            .add_user_buffer_items_impl_v2(test_key, v2_items)
            .await
            .unwrap();

        // Add V3 buffer items with String post_ids
        let v3_items = vec![
            BufferItemV3 {
                publisher_user_id: "buf_pub3".to_string(),
                post_id: "buffer_string_id".to_string(),
                video_id: "buf_vid3".to_string(),
                item_type: "video_viewed".to_string(),
                percent_watched: 75.0,
                user_id: "user3".to_string(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(base_time - 75),
            },
            BufferItemV3 {
                publisher_user_id: "buf_pub4".to_string(),
                post_id: "9003".to_string(),
                video_id: "buf_vid4".to_string(),
                item_type: "video_viewed".to_string(),
                percent_watched: 25.0,
                user_id: "user4".to_string(),
                timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(base_time - 25),
            },
        ];

        state
            .add_user_buffer_items_impl_v3(test_key, v3_items)
            .await
            .unwrap();

        // Sleep briefly to ensure items are in buffer
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Read all items using V3 impl method (which now uses resilient logic internally)
        let retrieved = state
            .get_user_buffer_items_by_timestamp_impl_v3(test_key, base_time)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 4, "Should retrieve all 4 buffer items");

        // Verify all post_ids are now Strings
        let post_ids: Vec<String> = retrieved.iter().map(|item| item.post_id.clone()).collect();
        assert!(post_ids.contains(&"9001".to_string()));
        assert!(post_ids.contains(&"9002".to_string()));
        assert!(post_ids.contains(&"buffer_string_id".to_string()));
        assert!(post_ids.contains(&"9003".to_string()));

        // Verify video_ids are preserved
        let video_ids: Vec<String> = retrieved.iter().map(|item| item.video_id.clone()).collect();
        assert!(video_ids.contains(&"buf_vid1".to_string()));
        assert!(video_ids.contains(&"buf_vid2".to_string()));
        assert!(video_ids.contains(&"buf_vid3".to_string()));
        assert!(video_ids.contains(&"buf_vid4".to_string()));

        // Test timestamp filtering - get only older items
        let filtered = state
            .get_user_buffer_items_by_timestamp_impl_v3(test_key, base_time - 60)
            .await
            .unwrap();
        assert_eq!(
            filtered.len(),
            2,
            "Should only get items older than base_time - 60"
        );

        // These should be the items with timestamps at base_time - 100 and base_time - 75
        let filtered_vids: Vec<String> =
            filtered.iter().map(|item| item.video_id.clone()).collect();
        assert!(filtered_vids.contains(&"buf_vid1".to_string()));
        assert!(filtered_vids.contains(&"buf_vid3".to_string()));

        // Clean up test key
        let _ = conn.del::<&str, ()>(test_key).await;
    }

    #[tokio::test]
    async fn test_is_user_history_plain_item_exists_v3_resilient() {
        let state = MLFeedCacheState::new().await;
        let key = "test_plain_exists_key";

        // Add V2 plain items (only video_id)
        let history_items_v2 = vec![
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub1".to_string(),
                canister_id: "can1".to_string(),
                post_id: 100,
                video_id: "video1".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: std::time::SystemTime::now(),
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItemV2 {
                publisher_user_id: "pub2".to_string(),
                canister_id: "can2".to_string(),
                post_id: 200,
                video_id: "video2".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: std::time::SystemTime::now(),
                percent_watched: 75.0,
            },
        ];

        state
            .add_user_history_plain_items_v2(key, history_items_v2)
            .await
            .unwrap();

        // Add V3 plain items
        let history_items_v3 = vec![
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub3".to_string(),
                canister_id: "can3".to_string(),
                post_id: "string_post_id".to_string(),
                video_id: "video3".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: std::time::SystemTime::now(),
                percent_watched: 90.0,
            },
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "pub4".to_string(),
                canister_id: "can4".to_string(),
                post_id: "400".to_string(),
                video_id: "video4".to_string(),
                item_type: "video_viewed".to_string(),
                timestamp: std::time::SystemTime::now(),
                percent_watched: 100.0,
            },
        ];

        state
            .add_user_history_plain_items_v3(key, history_items_v3)
            .await
            .unwrap();

        // Test existence check for V2 items using V3 method
        let item1 = PlainPostItemV3 {
            video_id: "video1".to_string(),
        };
        assert!(
            state
                .is_user_history_plain_item_exists_v3(key, item1)
                .await
                .unwrap(),
            "Should find V2 item (video1) using V3 method"
        );

        let item2 = PlainPostItemV3 {
            video_id: "video2".to_string(),
        };
        assert!(
            state
                .is_user_history_plain_item_exists_v3(key, item2)
                .await
                .unwrap(),
            "Should find V2 item (video2) using V3 method"
        );

        // Test existence check for V3 items
        let item3 = PlainPostItemV3 {
            video_id: "video3".to_string(),
        };
        assert!(
            state
                .is_user_history_plain_item_exists_v3(key, item3)
                .await
                .unwrap(),
            "Should find V3 item (video3)"
        );

        let item4 = PlainPostItemV3 {
            video_id: "video4".to_string(),
        };
        assert!(
            state
                .is_user_history_plain_item_exists_v3(key, item4)
                .await
                .unwrap(),
            "Should find V3 item (video4)"
        );

        // Test non-existent item
        let non_existent = PlainPostItemV3 {
            video_id: "non_existent_video".to_string(),
        };
        assert!(
            !state
                .is_user_history_plain_item_exists_v3(key, non_existent)
                .await
                .unwrap(),
            "Should not find non-existent item"
        );

        // Clean up
        let mut conn = state.redis_pool.get().await.unwrap();
        let _ = conn.del::<&str, ()>(key).await;
    }
}
