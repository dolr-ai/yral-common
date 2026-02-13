use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::LTX2_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// LTX-2 19B Distilled model - Self-hosted on Vast.ai H100
/// Supports text-to-video, image-to-video, and image+text-to-video
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Ltx2Model {
    pub prompt: String,
    pub image: Option<ImageData>,
}

impl VideoGenerator for Ltx2Model {
    fn model_id(&self) -> &'static str {
        "ltx2"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Ltx2
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        // At least one of prompt or image must be provided
        if self.prompt.is_empty() && self.image.is_none() {
            return Err(VideoGenError::InvalidInput(
                "Either prompt or image must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn get_prompt(&self) -> &str {
        &self.prompt
    }

    fn get_image(&self) -> Option<&ImageData> {
        self.image.as_ref()
    }

    fn get_image_mut(&mut self) -> Option<&mut ImageData> {
        self.image.as_mut()
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        None // Will be configured via env vars
    }

    fn supports_webhook_callbacks(&self) -> bool {
        true
    }
}

impl FlowControlFromEnv for Ltx2Model {
    fn env_prefix(&self) -> &'static str {
        "LTX2"
    }
}

impl Ltx2Model {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::ResolutionV2;

        // Validate parameters
        if unified.negative_prompt.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LTX-2 negative prompt is hardcoded and cannot be customized".to_string(),
            ));
        }

        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LTX-2 seed is not configurable".to_string(),
            ));
        }

        // Validate duration (only 5s supported - 121 frames at 24fps)
        if let Some(duration) = unified.duration_seconds {
            if duration != 5 {
                return Err(VideoGenError::InvalidInput(
                    "LTX-2 only supports 5 second duration".to_string(),
                ));
            }
        }

        // Validate resolution (720p or 1080p supported)
        if let Some(ref res) = unified.resolution {
            if *res != ResolutionV2::R720p && *res != ResolutionV2::R1080p {
                return Err(VideoGenError::InvalidInput(
                    "LTX-2 supports 720p and 1080p resolutions".to_string(),
                ));
            }
        }

        // At least prompt or image must be provided
        if unified.prompt.is_empty() && unified.image.is_none() {
            return Err(VideoGenError::InvalidInput(
                "Either prompt or image must be provided".to_string(),
            ));
        }

        Ok(VideoGenInput::Ltx2(Ltx2Model {
            prompt: unified.prompt,
            image: unified.image,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "ltx2".to_string(),
            name: "LTX-2 Video".to_string(),
            description: "Lightricks LTX-2 19B distilled model for fast, high-quality video generation. Supports text-to-video and image-to-video.".to_string(),
            cost: CostInfo::from_usd_cents(LTX2_COST_USD_CENTS),
            supports_image: true,
            supports_negative_prompt: false, // Hardcoded
            supports_audio: true,
            supports_audio_input: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio16x9, AspectRatioV2::Ratio9x16],
            allowed_resolutions: vec![ResolutionV2::R720p, ResolutionV2::R1080p],
            allowed_durations: vec![5],
            default_aspect_ratio: Some(AspectRatioV2::Ratio16x9),
            default_resolution: Some(ResolutionV2::R720p),
            default_duration: Some(5),
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/yral/favicon.svg".to_string()),
            ios_model_icon: Some("https://yral.com/img/yral/android-chrome-192x192.png".to_string()),
            extra_info: HashMap::from([
                ("architecture".to_string(), serde_json::json!("DiT")),
                ("model_size".to_string(), serde_json::json!("19B distilled")),
                ("inference_time".to_string(), serde_json::json!("~7 seconds (cached)")),
                ("hosting".to_string(), serde_json::json!("Self-hosted H100")),
            ]),
        }
    }
}
