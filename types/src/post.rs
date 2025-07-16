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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostItemV2 {
    pub publisher_user_id: Principal,
    pub canister_id: Principal,
    pub post_id: u64,
    pub video_id: String,
    pub is_nsfw: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeedRequestV2 {
    pub user_id: Principal,
    pub filter_results: Vec<PostItemV2>,
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeedResponseV2 {
    pub posts: Vec<PostItemV2>,
}
