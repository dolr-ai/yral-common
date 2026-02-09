use crate::generator::FlowControlFromEnv;
use crate::types::{VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::WAN2_5_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Wan25Model {
    pub prompt: String,
    /// Optional image for image-to-video generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<crate::types::ImageData>,
    // All other parameters are hardcoded in the provider implementation
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Wan25FastModel {
    pub prompt: String,
    /// Optional image for image-to-video generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<crate::types::ImageData>,
    // All other parameters are hardcoded in the provider implementation
}

impl VideoGenerator for Wan25Model {
    fn model_id(&self) -> &'static str {
        "wan2_5"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Wan25
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

    fn supports_webhook_callbacks(&self) -> bool {
        true
    }
}

impl FlowControlFromEnv for Wan25Model {
    fn env_prefix(&self) -> &'static str {
        "WAN2_5"
    }
}

impl VideoGenerator for Wan25FastModel {
    fn model_id(&self) -> &'static str {
        "wan2_5_fast"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Wan25Fast
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

    fn supports_webhook_callbacks(&self) -> bool {
        true
    }
}

impl FlowControlFromEnv for Wan25FastModel {
    fn env_prefix(&self) -> &'static str {
        "WAN2_5_FAST"
    }
}

impl Wan25Model {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::ResolutionV2;

        // Validate parameters that Wan 2.5 doesn't support in unified API
        if unified.negative_prompt.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 negative prompt is hardcoded and cannot be customized".to_string(),
            ));
        }

        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 seed is hardcoded to -1 (random)".to_string(),
            ));
        }

        // Validate duration (only 5s supported)
        if let Some(duration) = unified.duration_seconds {
            if duration != 5 {
                return Err(VideoGenError::InvalidInput(
                    "Wan 2.5 only supports 5 second duration".to_string(),
                ));
            }
        }

        // Validate resolution (only 720p supported)
        if unified.resolution.is_some() && unified.resolution != Some(ResolutionV2::R720p) {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 only supports 720p resolution".to_string(),
            ));
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        Ok(VideoGenInput::Wan25(Wan25Model {
            prompt: unified.prompt,
            image: unified.image,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "wan2_5".to_string(),
            name: "Wan 2.5".to_string(),
            description: "Alibaba's Wan 2.5 MoE model for cinematic 720p video generation with superior motion coherence. Supports image-to-video.".to_string(),
            cost: CostInfo::from_usd_cents(WAN2_5_COST_USD_CENTS),
            supports_image: true,
            supports_negative_prompt: false, // Hardcoded
            supports_audio: true,
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
            ]),
        }
    }
}

impl Wan25FastModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::ResolutionV2;

        // Validate parameters that Wan 2.5 Fast doesn't support in unified API
        if unified.negative_prompt.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 Fast negative prompt is hardcoded and cannot be customized".to_string(),
            ));
        }

        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 Fast seed is hardcoded to -1 (random)".to_string(),
            ));
        }

        // Validate duration (only 5s supported)
        if let Some(duration) = unified.duration_seconds {
            if duration != 5 {
                return Err(VideoGenError::InvalidInput(
                    "Wan 2.5 Fast only supports 5 second duration".to_string(),
                ));
            }
        }

        // Validate resolution (only 720p supported)
        if unified.resolution.is_some() && unified.resolution != Some(ResolutionV2::R720p) {
            return Err(VideoGenError::InvalidInput(
                "Wan 2.5 Fast only supports 720p resolution".to_string(),
            ));
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        Ok(VideoGenInput::Wan25Fast(Wan25FastModel {
            prompt: unified.prompt,
            image: unified.image,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "wan2_5_fast".to_string(),
            name: "Wan 2.5 Fast".to_string(),
            description: "Alibaba's Wan 2.5 Fast model for quick 720p video generation with optimized speed. Supports image-to-video.".to_string(),
            cost: CostInfo::from_usd_cents(WAN2_5_COST_USD_CENTS),
            supports_image: true,
            supports_negative_prompt: false, // Hardcoded
            supports_audio: true,
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
                ("speed".to_string(), serde_json::json!("optimized")),
            ]),
        }
    }
}
