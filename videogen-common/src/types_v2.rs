use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString};
use utoipa::ToSchema;
use yral_types::delegated_identity::DelegatedIdentityWire;

use crate::types::{ImageData, TokenType, VideoGenRequestKey};

/// Aspect ratio options for video generation in v2 API
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, ToSchema, CandidType, Display, EnumString,
)]
pub enum AspectRatioV2 {
    #[serde(rename = "16:9")]
    #[strum(serialize = "16:9")]
    Ratio16x9,
    #[serde(rename = "9:16")]
    #[strum(serialize = "9:16")]
    Ratio9x16,
    #[serde(rename = "1:1")]
    #[strum(serialize = "1:1")]
    Ratio1x1,
    #[serde(rename = "4:3")]
    #[strum(serialize = "4:3")]
    Ratio4x3,
    #[serde(rename = "3:4")]
    #[strum(serialize = "3:4")]
    Ratio3x4,
}

/// Resolution options for video generation in v2 API
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, ToSchema, CandidType, Display, EnumString,
)]
pub enum ResolutionV2 {
    #[serde(rename = "540p")]
    #[strum(serialize = "540p")]
    R540p,
    #[serde(rename = "720p")]
    #[strum(serialize = "720p")]
    R720p,
    #[serde(rename = "1080p")]
    #[strum(serialize = "1080p")]
    R1080p,
    #[serde(rename = "4k")]
    #[strum(serialize = "4k")]
    R4k,
}

/// Unified request structure for v2 API
/// All parameters are optional except prompt and model_id
/// The backend will adapt this to model-specific structures
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenRequestV2 {
    #[serde(rename = "user_id")]
    #[schema(value_type = String, example = "xkbqi-2qaaa-aaaah-qbpqq-cai")]
    pub principal: Principal,

    /// The prompt for video generation
    #[schema(example = "A cat playing piano in a jazz club")]
    pub prompt: String,

    /// The model to use (e.g., "veo3", "veo3_fast", "lumalabs", "falai")
    #[schema(example = "veo3")]
    pub model_id: String,

    /// Token type for payment
    #[serde(default)]
    pub token_type: TokenType,

    // Common optional parameters
    /// Negative prompt (if supported by model)
    #[schema(example = "blurry, low quality")]
    pub negative_prompt: Option<String>,

    /// Optional input image for image-to-video
    pub image: Option<ImageData>,

    /// Aspect ratio
    #[schema(example = "16:9")]
    pub aspect_ratio: Option<AspectRatioV2>,

    /// Duration in seconds
    #[schema(example = 5)]
    pub duration_seconds: Option<u8>,

    /// Resolution
    #[schema(example = "1080p")]
    pub resolution: Option<ResolutionV2>,

    /// Whether to generate audio
    #[schema(example = true)]
    pub generate_audio: Option<bool>,

    /// Random seed for reproducibility
    #[schema(example = 42)]
    pub seed: Option<u64>,

    /// Additional model-specific parameters
    /// This allows for flexibility without breaking the API
    #[serde(default)]
    pub extra_params: HashMap<String, serde_json::Value>,
}

/// Request with delegated identity for v2 API
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenRequestWithIdentityV2 {
    pub request: VideoGenRequestV2,
    #[schema(value_type = Object)]
    pub delegated_identity: DelegatedIdentityWire,
}

/// Cost information in multiple currencies
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct CostInfo {
    /// Cost in USD cents
    #[schema(example = 10)]
    pub usd_cents: u64,

    /// Cost in DOLR (e8s - smallest unit)
    #[schema(example = 2000000000)]
    pub dolr: u64,

    /// Cost in SATS (smallest unit)
    #[schema(example = 100)]
    pub sats: u64,
}

impl CostInfo {
    /// Create cost info from USD cents using TOKEN_COST_CONFIG
    pub fn from_usd_cents(usd_cents: u64) -> Self {
        use crate::token_costs::TOKEN_COST_CONFIG;
        use crate::types::TokenType;

        Self {
            usd_cents,
            dolr: TOKEN_COST_CONFIG.convert_usd_to_token(usd_cents, &TokenType::Dolr),
            sats: TOKEN_COST_CONFIG.convert_usd_to_token(usd_cents, &TokenType::Sats),
        }
    }
}

/// Provider information for the metadata API
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct ProviderInfo {
    /// Unique identifier for the model
    #[schema(example = "veo3")]
    pub id: String,

    /// Display name
    #[schema(example = "Veo3")]
    pub name: String,

    /// Description of the model
    #[schema(example = "Google's advanced video generation model")]
    pub description: String,

    /// Cost in multiple currencies
    pub cost: CostInfo,

    // Capabilities
    /// Whether the model supports image input
    pub supports_image: bool,

    /// Whether the model supports negative prompts
    pub supports_negative_prompt: bool,

    /// Whether the model can generate audio
    pub supports_audio: bool,

    /// Whether the model supports seed for reproducibility
    pub supports_seed: bool,

    // Allowed values
    /// List of supported aspect ratios
    pub allowed_aspect_ratios: Vec<AspectRatioV2>,

    /// List of supported resolutions
    pub allowed_resolutions: Vec<ResolutionV2>,

    /// List of allowed duration values in seconds
    #[schema(example = json!([5, 8]))]
    pub allowed_durations: Vec<u8>,

    // Defaults
    /// Default aspect ratio if not specified
    pub default_aspect_ratio: Option<AspectRatioV2>,

    /// Default resolution if not specified
    pub default_resolution: Option<ResolutionV2>,

    /// Default duration if not specified
    #[schema(example = 5)]
    pub default_duration: u8,

    // Status
    /// Whether the model is currently available
    pub is_available: bool,

    /// Whether this is an internal model (e.g., for testing)
    #[serde(default)]
    pub is_internal: bool,

    /// Path to model icon
    #[schema(example = "/img/ai-models/veo3.svg")]
    pub model_icon: Option<String>,

    /// Path to model icon
    #[schema(example = "/img/ai-models/veo3.svg")]
    pub ios_model_icon: Option<String>,

    /// Additional model-specific information
    #[serde(default)]
    pub extra_info: HashMap<String, serde_json::Value>,
}

/// Response containing all available providers
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderInfo>,
    /// Version of the provider schema for client compatibility
    #[schema(example = "1.0.0")]
    pub schema_version: String,
}

/// V2 queued response (same as v1 but included for completeness)
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct VideoGenQueuedResponseV2 {
    pub operation_id: String,
    pub provider: String,
    pub request_key: VideoGenRequestKey,
}
