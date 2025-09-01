use anyhow::Result;
use ml_feed_cache::{consts, MLFeedCacheState};
use std::collections::HashSet;
use std::env;

async fn backfill_watched_video_ids(
    state: &MLFeedCacheState,
    user_id: &str,
    is_nsfw: bool,
) -> Result<u64> {
    let history_suffixes = if is_nsfw {
        vec![consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2, consts::USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2]
    } else {
        vec![consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2, consts::USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2]
    };

    let mut all_video_ids = HashSet::new();

    // Collect from history
    for suffix in history_suffixes {
        let key = format!("{}{}", user_id, suffix);
        if let Ok(items) = state.get_watch_history_items_v3_resilient(&key, 0, consts::MAX_WATCH_HISTORY_CACHE_LEN - 1).await {
            for item in items {
                all_video_ids.insert(item.video_id);
            }
        }
    }

    // Collect from plain post items
    let plain_key = format!("{}{}", user_id, consts::USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2);
    if let Ok(items) = state.get_plain_post_items_v3(&plain_key, 0, consts::MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN - 1).await {
        for item in items {
            all_video_ids.insert(item.video_id);
        }
    }

    let count = all_video_ids.len() as u64;
    if !all_video_ids.is_empty() {
        let set_key = format!("{}{}", user_id, if is_nsfw {
            consts::USER_WATCHED_VIDEO_IDS_SET_NSFW_SUFFIX_V2
        } else {
            consts::USER_WATCHED_VIDEO_IDS_SET_CLEAN_SUFFIX_V2
        });
        
        state.add_watched_video_ids_to_set(&set_key, all_video_ids.into_iter().collect()).await?;
        println!("Added {} video IDs for user {} (nsfw: {})", count, user_id, is_nsfw);
    }

    Ok(count)
}

async fn backfill_all_users(state: &MLFeedCacheState) -> Result<()> {
    let mut conn = state.redis_pool.get().await?;
    let patterns = vec![
        format!("*{}", consts::USER_WATCH_HISTORY_CLEAN_SUFFIX_V2),
        format!("*{}", consts::USER_WATCH_HISTORY_NSFW_SUFFIX_V2),
    ];

    let mut processed = HashSet::new();

    for pattern in patterns {
        let mut cursor = 0u64;
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor).arg("MATCH").arg(&pattern).arg("COUNT").arg(100)
                .query_async(&mut *conn).await?;

            for key in keys {
                let user_id = key.rsplit_once('_').map(|(p, _)| p).unwrap_or(&key).to_string();
                let is_nsfw = key.contains("_nsfw");
                let user_key = format!("{}_{}", user_id, is_nsfw);
                
                if processed.insert(user_key) {
                    backfill_watched_video_ids(state, &user_id, is_nsfw).await.ok();
                }
            }

            cursor = new_cursor;
            if cursor == 0 { break; }
        }
    }

    println!("Backfilled {} users", processed.len());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    if env::var("ML_FEED_CACHE_REDIS_URL").is_err() || env::var("ML_FEED_CACHE_MEMORYSTORE_URL").is_err() {
        eprintln!("Required environment variables not set");
        std::process::exit(1);
    }

    let state = MLFeedCacheState::new().await;
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        let user_id = &args[1];
        let is_nsfw = args.len() > 2 && args[2] == "nsfw";
        backfill_watched_video_ids(&state, user_id, is_nsfw).await?;
    } else {
        backfill_all_users(&state).await?;
    }

    Ok(())
}