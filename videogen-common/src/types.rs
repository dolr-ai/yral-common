use crate::models::{FalAiModel, IntTestModel, LumaLabsModel, Veo3FastModel, Veo3Model};
use candid::{CandidType, Principal};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[cfg(feature = "ic")]
use yral_identity::Signature;

/// Core trait for video generation models
#[enum_dispatch]
pub trait VideoGenerator {
    /// Get the model name for rate limiting and identification
    fn model_name(&self) -> &'static str;

    /// Get the provider for this model
    fn provider(&self) -> VideoGenProvider;

    /// Validate the input parameters
    fn validate_input(&self) -> Result<(), VideoGenError>;

    /// Get the prompt text
    fn get_prompt(&self) -> &str;

    /// Get the optional input image
    fn get_image(&self) -> Option<&ImageInput>;

    /// Get flow control key for Qstash rate limiting
    fn flow_control_key(&self) -> String {
        format!("VIDEOGEN_{}", self.model_name())
    }

    /// Get flow control configuration (rate_per_minute, parallelism)
    fn flow_control_config(&self) -> Option<(u32, u32)> {
        None // Default: no flow control
    }
}

// Request wrapper that includes user_id for rate limiting
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct VideoGenRequest {
    #[serde(rename = "user_id")]
    #[schema(value_type = String, example = "xkbqi-2qaaa-aaaah-qbpqq-cai")]
    pub principal: Principal,
    #[serde(flatten)]
    pub input: VideoGenInput,
}

#[enum_dispatch(VideoGenerator)]
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
#[serde(tag = "provider", content = "data")]
pub enum VideoGenInput {
    Veo3(Veo3Model),
    Veo3Fast(Veo3FastModel),
    FalAi(FalAiModel),
    LumaLabs(LumaLabsModel),
    IntTest(IntTestModel),
}

// VideoGenInput now gets model_name() and other methods from VideoGenerator trait via enum_dispatch

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema, CandidType, strum_macros::Display,
)]
pub enum VideoGenProvider {
    Veo3,
    Veo3Fast,
    FalAi,
    LumaLabs,
    IntTest,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct ImageInput {
    #[schema(
        example = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
    )]
    pub data: String, // Base64 encoded image data
    #[schema(example = "image/png")]
    pub mime_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, ToSchema, CandidType)]
pub enum Veo3AspectRatio {
    #[serde(rename = "16:9")]
    Ratio16x9,
    #[serde(rename = "9:16")]
    Ratio9x16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, ToSchema, CandidType)]
pub enum LumaLabsResolution {
    #[serde(rename = "540p")]
    R540p,
    #[serde(rename = "720p")]
    R720p,
    #[serde(rename = "1080p")]
    R1080p,
    #[serde(rename = "4k")]
    R4k,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, ToSchema, CandidType)]
pub enum LumaLabsDuration {
    #[serde(rename = "5s")]
    D5s,
    #[serde(rename = "9s")]
    D9s,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct VideoGenRequestKey {
    #[schema(value_type = String, example = "xkbqi-2qaaa-aaaah-qbpqq-cai")]
    pub principal: Principal,
    pub counter: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenResponse {
    pub operation_id: String,
    pub video_url: String,
    pub provider: String,
}

/// Initial response when video generation is queued
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenQueuedResponse {
    pub operation_id: String,
    pub provider: String,
    pub request_key: VideoGenRequestKey,
}

// Request with signature for authentication
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenRequestWithSignature {
    pub request: VideoGenRequest,
    #[cfg(feature = "ic")]
    #[schema(value_type = Object)]
    pub signature: Signature,
    #[cfg(not(feature = "ic"))]
    #[schema(value_type = Object)]
    pub signature: serde_json::Value,
}

#[cfg(feature = "ic")]
impl VideoGenRequestWithSignature {
    pub fn new_with_signature(request: VideoGenRequest, signature: Signature) -> Self {
        Self { request, signature }
    }

    pub fn get_signature(&self) -> &Signature {
        &self.signature
    }
}

#[cfg(not(feature = "ic"))]
impl VideoGenRequestWithSignature {
    pub fn get_signature(&self) -> Result<serde_json::Value, serde_json::Error> {
        Ok(self.signature.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, thiserror::Error, ToSchema)]
pub enum VideoGenError {
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Authentication failed")]
    AuthError,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Invalid signature")]
    InvalidSignature,
}
