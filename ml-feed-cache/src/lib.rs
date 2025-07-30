use std::time::{SystemTime, UNIX_EPOCH};

use ::types::post::PostItemV2;
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

pub mod consts;
pub mod types;
pub mod types_v2;

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

    pub async fn add_user_buffer_items_v2(
        &self,
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
            conn.zadd_multiple::<&str, f64, BufferItemV2, ()>(USER_HOTORNOT_BUFFER_KEY_V2, chunk)
                .await?;
        }

        Ok(())
    }

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
}
