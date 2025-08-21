use crate::generator::FlowControlFromEnv;
use crate::types::{
    ImageData, LumaLabsDuration, LumaLabsResolution,
    VideoGenProvider, VideoGenerator,
};
// VideoModel and ModelMetadata have been removed
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::RAY2FLASH_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct LumaLabsModel {
    pub prompt: String,
    pub image: Option<ImageData>,
    pub resolution: LumaLabsResolution,
    pub duration: LumaLabsDuration,
    pub aspect_ratio: Option<String>,
    pub loop_video: bool,
}

impl VideoGenerator for LumaLabsModel {
    fn model_id(&self) -> &'static str {
        "ray2flash"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::LumaLabs
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Validate aspect ratio format if provided
        if let Some(aspect_ratio) = &self.aspect_ratio {
            if !aspect_ratio.contains(':') {
                return Err(VideoGenError::InvalidInput(
                    "Aspect ratio must be in format 'width:height'".to_string(),
                ));
            }
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
        // No flow control for test model
        None
    }
}

impl FlowControlFromEnv for LumaLabsModel {
    fn env_prefix(&self) -> &'static str {
        "LUMALABS"
    }
}

// VideoModel has been removed - model info is now fetched dynamically via API
// static LUMALABS_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
//     id: "ray2flash".to_string(),
//     name: "Ray2Flash".to_string(),
//     description: "LumaLabs' fast AI video generation model".to_string(),
//     cost_usd_cents: RAY2FLASH_COST_USD_CENTS,
//     supports_image: true,
//     provider: VideoGenProvider::LumaLabs,
//     max_duration_seconds: 9,
//     supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
//     model_icon: Some("https://yral.com/img/ai-models/lumalabs.png".to_string()),
//     is_available: true,
// });

// impl ModelMetadata for LumaLabsModel {
//     fn model_info() -> &'static VideoModel {
//         &LUMALABS_MODEL_INFO
//     }
// }

impl LumaLabsModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::ResolutionV2;

        // Parse resolution
        let resolution = match unified.resolution {
            Some(ResolutionV2::R540p) => LumaLabsResolution::R540p,
            Some(ResolutionV2::R720p) => LumaLabsResolution::R720p,
            Some(ResolutionV2::R1080p) | None => LumaLabsResolution::R1080p, // default
            Some(ResolutionV2::R4k) => LumaLabsResolution::R4k,
        };

        // Parse duration
        let duration = match unified.duration_seconds {
            Some(5) | None => LumaLabsDuration::D5s, // default
            Some(9) => LumaLabsDuration::D9s,
            Some(other) => {
                return Err(VideoGenError::InvalidInput(format!(
                    "LumaLabs only supports 5 or 9 second durations, got: {}",
                    other
                )))
            }
        };

        // Validate before creating (do this first before moving values)
        Self::validate_unified_parameters(&unified)?;

        // Get aspect ratio (LumaLabs uses string format)
        let aspect_ratio = unified
            .aspect_ratio
            .map(|ar| ar.to_string())
            .or_else(|| Some("16:9".to_string()));

        // Check for loop_video in extra_params
        let loop_video = false;

        Ok(VideoGenInput::LumaLabs(LumaLabsModel {
            prompt: unified.prompt,
            image: unified.image,
            resolution,
            duration,
            aspect_ratio,
            loop_video,
        }))
    }

    /// Validate parameters from unified request
    pub fn validate_unified_parameters(
        unified: &crate::types_v2::VideoGenRequestV2,
    ) -> Result<(), VideoGenError> {
        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // LumaLabs doesn't support negative prompt
        if unified.negative_prompt.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LumaLabs does not support negative prompts".to_string(),
            ));
        }

        // LumaLabs doesn't support seed
        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LumaLabs does not support seed parameter".to_string(),
            ));
        }

        // LumaLabs doesn't support audio generation parameter
        if unified.generate_audio.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LumaLabs does not have configurable audio generation".to_string(),
            ));
        }

        // Aspect ratio validation no longer needed - handled by enum

        Ok(())
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "lumalabs".to_string(),
            name: "Luma Labs".to_string(),
            description: "Luma Labs Dream Machine for high-quality video generation".to_string(),
            cost: CostInfo::from_usd_cents(RAY2FLASH_COST_USD_CENTS),
            supports_image: true,
            supports_negative_prompt: false,
            supports_audio: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![
                AspectRatioV2::Ratio16x9,
                AspectRatioV2::Ratio9x16,
                AspectRatioV2::Ratio1x1,
                AspectRatioV2::Ratio4x3,
                AspectRatioV2::Ratio3x4,
            ],
            allowed_resolutions: vec![
                ResolutionV2::R540p,
                ResolutionV2::R720p,
                ResolutionV2::R1080p,
                ResolutionV2::R4k,
            ],
            allowed_durations: vec![9],
            default_aspect_ratio: Some(AspectRatioV2::Ratio16x9),
            default_resolution: Some(ResolutionV2::R1080p),
            default_duration: 9,
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/ai-models/lumalabs.png".to_string()),
            extra_info: HashMap::new(),
        }
    }
}
