use ic_agent::export::Principal;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use utoipa::ToSchema;

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

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedRequestV2 {
    pub user_id: String,
    pub canister_id: String,
    pub filter_results: Vec<String>, // List of video IDs to filter results
    pub num_results: u32,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct PostItemV2 {
    pub publisher_user_id: String,
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub nsfw_probability: f32,
}

impl Eq for PostItemV2 {}

impl PartialEq for PostItemV2 {
    fn eq(&self, other: &Self) -> bool {
        self.publisher_user_id == other.publisher_user_id && self.post_id == other.post_id
    }
}

impl Hash for PostItemV2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.publisher_user_id.hash(state);
        self.post_id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct FeedResponseV2 {
    pub posts: Vec<PostItemV2>,
}
