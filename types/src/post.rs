use ic_agent::export::Principal;
use serde::{Deserialize, Serialize};

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
