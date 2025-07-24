use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[cfg(feature = "ic")]
use yral_identity::Signature;

// Request wrapper that includes user_id for rate limiting
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct VideoGenRequest {
    #[serde(rename = "user_id")]
    #[schema(value_type = String, example = "xkbqi-2qaaa-aaaah-qbpqq-cai")]
    pub principal: Principal,
    #[serde(flatten)]
    pub input: VideoGenInput,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
#[serde(tag = "provider", content = "data")]
pub enum VideoGenInput {
    Veo3 {
        prompt: String,
        negative_prompt: Option<String>,
        image: Option<ImageInput>,
        aspect_ratio: Veo3AspectRatio,
        duration_seconds: u8,
        generate_audio: bool,
    },
    FalAi {
        prompt: String,
        model: String,
        seed: Option<u64>,
        num_frames: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct ImageInput {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub enum Veo3AspectRatio {
    #[serde(rename = "16:9")]
    Ratio16x9,
    #[serde(rename = "9:16")]
    Ratio9x16,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenResponse {
    pub operation_id: String,
    pub video_url: String,
    pub provider: String,
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
