use crate::generator::FlowControlFromEnv;
use crate::types::{VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::WAN2_2_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Wan22Model {
    pub prompt: String,
    pub image: Option<crate::types::ImageData>, // For I2V mode
    pub is_i2v: bool, // Flag to determine T2V or I2V endpoint
}

impl VideoGenerator for Wan22Model {
    fn model_id(&self) -> &'static str {
        "wan2_2"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Wan22
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn get_prompt(&self) -> &str {
        &self.prompt
    }

    fn get_image(&self) -> Option<&crate::types::ImageData> {
        self.image.as_ref()
    }

    fn get_image_mut(&mut self) -> Option<&mut crate::types::ImageData> {
        self.image.as_mut()
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        None // Will be configured via env vars
    }
}

impl FlowControlFromEnv for Wan22Model {
    fn env_prefix(&self) -> &'static str {
        "WAN2_2"
    }
}

impl Wan22Model {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::ResolutionV2;

        // Determine if I2V mode based on image presence
        let is_i2v = unified.image.is_some();

        // Note: negative prompt and seed are hardcoded in the implementation

        // Validate duration (only 5s supported)
        if let Some(duration) = unified.duration_seconds {
            if duration != 5 {
                return Err(VideoGenError::InvalidInput(
                    "Wan 2.2 only supports 5 second duration".to_string(),
                ));
            }
        }

        // Validate resolution (only 720p supported)
        if unified.resolution.is_some() && unified.resolution != Some(ResolutionV2::R720p) {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.2 only supports 720p resolution".to_string(),
            ));
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        Ok(VideoGenInput::Wan22(Wan22Model {
            prompt: unified.prompt,
            image: unified.image,
            is_i2v,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "wan2_2".to_string(),
            name: "Wan 2.2 T2V".to_string(),
            description: "Alibaba's Wan 2.2 MoE model for cinematic 720p text-to-video generation with superior motion coherence".to_string(),
            cost: CostInfo::from_usd_cents(WAN2_2_COST_USD_CENTS),
            supports_image: true, // Supports I2V mode
            supports_negative_prompt: false, // Hardcoded
            supports_audio: false,
            supports_audio_input: false,
            supports_seed: false, // Always -1 (random)
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio9x16], // 720x1280
            allowed_resolutions: vec![ResolutionV2::R720p],
            allowed_durations: vec![5],
            default_aspect_ratio: Some(AspectRatioV2::Ratio9x16),
            default_resolution: Some(ResolutionV2::R720p),
            default_duration: Some(5),
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/yral/favicon.svg".to_string()),
            ios_model_icon: Some("https://yral.com/img/yral/android-chrome-192x192.png".to_string()),
            extra_info: HashMap::from([
                ("architecture".to_string(), serde_json::json!("MoE Flow-Matching")),
                ("model_size".to_string(), serde_json::json!("14B")),
                ("supports_modes".to_string(), serde_json::json!(["t2v", "i2v"])),
            ]),
        }
    }
}
