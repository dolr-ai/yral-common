// Cache length limits
pub const MAX_WATCH_HISTORY_CACHE_LEN: u64 = 10000;
pub const MAX_SUCCESS_HISTORY_CACHE_LEN: u64 = 10000;
pub const MAX_GLOBAL_CACHE_LEN: u64 = 3000;
pub const MAX_USER_CACHE_LEN: u64 = 1000;
pub const MAX_HISTORY_PLAIN_POST_ITEM_CACHE_LEN: u64 = 10000;

// Global cache keys - v1
pub const GLOBAL_CACHE_CLEAN_KEY: &str = "global_cache_clean";
pub const GLOBAL_CACHE_NSFW_KEY: &str = "global_cache_nsfw";
pub const GLOBAL_CACHE_MIXED_KEY: &str = "global_cache_mixed";

// Global cache keys - v2
pub const GLOBAL_CACHE_CLEAN_KEY_V2: &str = "global_cache_clean_v2";
pub const GLOBAL_CACHE_NSFW_KEY_V2: &str = "global_cache_nsfw_v2";
pub const GLOBAL_CACHE_MIXED_KEY_V2: &str = "global_cache_mixed_v2";

// User history suffixes - v1
pub const USER_WATCH_HISTORY_CLEAN_SUFFIX: &str = "_watch_clean";
pub const USER_SUCCESS_HISTORY_CLEAN_SUFFIX: &str = "_success_clean";
pub const USER_WATCH_HISTORY_NSFW_SUFFIX: &str = "_watch_nsfw";
pub const USER_SUCCESS_HISTORY_NSFW_SUFFIX: &str = "_success_nsfw";

// User history suffixes - v2
pub const USER_WATCH_HISTORY_CLEAN_SUFFIX_V2: &str = "_watch_clean_v2";
pub const USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2: &str = "_success_clean_v2";
pub const USER_WATCH_HISTORY_NSFW_SUFFIX_V2: &str = "_watch_nsfw_v2";
pub const USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2: &str = "_success_nsfw_v2";

// User history plain post item suffixes - v1
pub const USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX: &str = "_watch_plain_post_item";
pub const USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX: &str = "_like_plain_post_item";

// User history plain post item suffixes - v2
pub const USER_WATCH_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2: &str = "_watch_plain_post_item_v2";
pub const USER_LIKE_HISTORY_PLAIN_POST_ITEM_SUFFIX_V2: &str = "_like_plain_post_item_v2";

// User hotornot buffer keys - v1
pub const USER_HOTORNOT_BUFFER_KEY: &str = "user_hotornot_buffer";

// User hotornot buffer keys - v2
pub const USER_HOTORNOT_BUFFER_KEY_V2: &str = "user_hotornot_buffer_v2";

// User hotornot buffer keys - v3
pub const USER_HOTORNOT_BUFFER_KEY_V3: &str = "user_hotornot_buffer_v3";

// User cache suffixes - v1
pub const USER_CACHE_CLEAN_SUFFIX: &str = "_cache_clean";
pub const USER_CACHE_NSFW_SUFFIX: &str = "_cache_nsfw";
pub const USER_CACHE_MIXED_SUFFIX: &str = "_cache_mixed";

// User cache suffixes - v2
pub const USER_CACHE_CLEAN_SUFFIX_V2: &str = "_cache_clean_v2";
pub const USER_CACHE_NSFW_SUFFIX_V2: &str = "_cache_nsfw_v2";
pub const USER_CACHE_MIXED_SUFFIX_V2: &str = "_cache_mixed_v2";
