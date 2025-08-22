use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, Veo3AspectRatio, VideoGenProvider, VideoGenerator};
// VideoModel and ModelMetadata have been removed
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::VEO3_COST_USD_CENTS;
use global_constants::VEO3_FAST_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Veo3Model {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub image: Option<ImageData>,
    pub aspect_ratio: Veo3AspectRatio,
    pub duration_seconds: u8,
    pub generate_audio: bool,
}

impl VideoGenerator for Veo3Model {
    fn model_id(&self) -> &'static str {
        "veo3"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Veo3
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if self.duration_seconds == 0 || self.duration_seconds > 8 {
            return Err(VideoGenError::InvalidInput(
                "Duration must be between 1 and 8 seconds".to_string(),
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
        // No flow control for test model
        None
    }
}

impl FlowControlFromEnv for Veo3Model {
    fn env_prefix(&self) -> &'static str {
        "VEO3"
    }
}

// VideoModel has been removed - model info is now fetched dynamically via API
// static VEO3_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
//     id: "veo3".to_string(),
//     name: "Veo3".to_string(),
//     description: "Google's advanced video generation model".to_string(),
//     cost_usd_cents: VEO3_COST_USD_CENTS,
//     supports_image: false,
//     provider: VideoGenProvider::Veo3,
//     max_duration_seconds: 8,
//     supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9, Veo3AspectRatio::Ratio9x16],
//     model_icon: Some("https://yral.com/img/ai-models/veo3.svg".to_string()),
//     is_available: true,
// });

// impl ModelMetadata for Veo3Model {
//     fn model_info() -> &'static VideoModel {
//         &VEO3_MODEL_INFO
//     }
// }

impl Veo3Model {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::AspectRatioV2;

        // Parse aspect ratio
        let aspect_ratio = match unified.aspect_ratio {
            Some(AspectRatioV2::Ratio16x9) | None => Veo3AspectRatio::Ratio16x9,
            Some(AspectRatioV2::Ratio9x16) => Veo3AspectRatio::Ratio9x16,
            Some(other) => {
                return Err(VideoGenError::InvalidInput(format!(
                    "Veo3 does not support {} aspect ratio. Supported: 16:9, 9:16",
                    other
                )))
            }
        };

        // Get duration (default to 8 seconds)
        let duration_seconds = unified.duration_seconds.unwrap_or(8);

        // Get audio generation flag (default to true)
        let generate_audio = unified.generate_audio.unwrap_or(true);

        // Validate before creating
        Self::validate_unified_parameters(&unified)?;

        Ok(VideoGenInput::Veo3(Veo3Model {
            prompt: unified.prompt,
            negative_prompt: unified.negative_prompt,
            image: unified.image,
            aspect_ratio,
            duration_seconds,
            generate_audio,
        }))
    }

    /// Validate parameters from unified request
    pub fn validate_unified_parameters(
        unified: &crate::types_v2::VideoGenRequestV2,
    ) -> Result<(), VideoGenError> {
        // Validate duration
        if let Some(duration) = unified.duration_seconds {
            if duration == 0 || duration > 8 {
                return Err(VideoGenError::InvalidInput(
                    "Duration must be between 1 and 8 seconds for Veo3".to_string(),
                ));
            }
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Veo3 doesn't support seed
        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3 does not support seed parameter".to_string(),
            ));
        }

        // Veo3 doesn't support resolution parameter (uses aspect ratio instead)
        if unified.resolution.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3 does not support resolution parameter. Use aspect_ratio instead".to_string(),
            ));
        }

        // Veo3 doesn't support image input
        if unified.image.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3 does not support image input".to_string(),
            ));
        }

        Ok(())
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "veo3".to_string(),
            name: "Veo3".to_string(),
            description: "Google's advanced video generation model".to_string(),
            cost: CostInfo::from_usd_cents(VEO3_COST_USD_CENTS),
            supports_image: false,
            supports_negative_prompt: true,
            supports_audio: true,
            supports_audio_input: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio16x9, AspectRatioV2::Ratio9x16],
            allowed_resolutions: vec![], // Not applicable for Veo3
            allowed_durations: vec![8],
            default_aspect_ratio: Some(AspectRatioV2::Ratio16x9),
            default_resolution: None,
            default_duration: Some(8),
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/ai-models/veo3.svg".to_string()),
            ios_model_icon: Some("https://yral.com/img/ai-models/veo3.png".to_string()),
            extra_info: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct Veo3FastModel {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub image: Option<ImageData>,
    pub aspect_ratio: Veo3AspectRatio,
    pub duration_seconds: u8,
    pub generate_audio: bool,
}

impl VideoGenerator for Veo3FastModel {
    fn model_id(&self) -> &'static str {
        "veo3_fast"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::Veo3Fast
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if self.duration_seconds == 0 || self.duration_seconds > 8 {
            return Err(VideoGenError::InvalidInput(
                "Duration must be between 1 and 8 seconds".to_string(),
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
        // No flow control for test model
        None
    }
}

impl FlowControlFromEnv for Veo3FastModel {
    fn env_prefix(&self) -> &'static str {
        "VEO3_FAST"
    }
}

// VideoModel has been removed - model info is now fetched dynamically via API
// static VEO3_FAST_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
//     id: "veo3_fast".to_string(),
//     name: "Veo3 Fast".to_string(),
//     description: "Google Veo3 Faster and cheaper".to_string(),
//     cost_usd_cents: VEO3_FAST_COST_USD_CENTS,
//     supports_image: false,
//     provider: VideoGenProvider::Veo3Fast,
//     max_duration_seconds: 8,
//     supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
//     model_icon: Some("https://yral.com/img/ai-models/veo3.svg".to_string()),
//     is_available: true,
// });

// impl ModelMetadata for Veo3FastModel {
//     fn model_info() -> &'static VideoModel {
//         &VEO3_FAST_MODEL_INFO
//     }
// }

impl Veo3FastModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        use crate::types_v2::AspectRatioV2;

        // Parse aspect ratio (Veo3Fast only supports 16:9)
        let aspect_ratio = match unified.aspect_ratio {
            Some(AspectRatioV2::Ratio16x9) | None => Veo3AspectRatio::Ratio16x9,
            Some(other) => {
                return Err(VideoGenError::InvalidInput(format!(
                    "Veo3Fast only supports 16:9 aspect ratio, got: {}",
                    other
                )))
            }
        };

        // Get duration (default to 8 seconds)
        let duration_seconds = unified.duration_seconds.unwrap_or(8);

        // Get audio generation flag (default to true)
        let generate_audio = unified.generate_audio.unwrap_or(true);

        // Validate before creating
        Self::validate_unified_parameters(&unified)?;

        Ok(VideoGenInput::Veo3Fast(Veo3FastModel {
            prompt: unified.prompt,
            negative_prompt: unified.negative_prompt,
            image: unified.image,
            aspect_ratio,
            duration_seconds,
            generate_audio,
        }))
    }

    /// Validate parameters from unified request
    pub fn validate_unified_parameters(
        unified: &crate::types_v2::VideoGenRequestV2,
    ) -> Result<(), VideoGenError> {
        // Validate duration
        if let Some(duration) = unified.duration_seconds {
            if duration == 0 || duration > 8 {
                return Err(VideoGenError::InvalidInput(
                    "Duration must be between 1 and 8 seconds for Veo3Fast".to_string(),
                ));
            }
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Veo3Fast doesn't support seed
        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3Fast does not support seed parameter".to_string(),
            ));
        }

        // Veo3Fast doesn't support resolution parameter
        if unified.resolution.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3Fast does not support resolution parameter".to_string(),
            ));
        }

        // Veo3Fast doesn't support image input
        if unified.image.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Veo3Fast does not support image input".to_string(),
            ));
        }

        Ok(())
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "veo3_fast".to_string(),
            name: "Veo3 Fast".to_string(),
            description: "Google Veo3 - Faster and cheaper".to_string(),
            cost: CostInfo::from_usd_cents(VEO3_FAST_COST_USD_CENTS),
            supports_image: false,
            supports_negative_prompt: true,
            supports_audio: true,
            supports_audio_input: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio16x9, AspectRatioV2::Ratio9x16],
            allowed_resolutions: vec![],
            allowed_durations: vec![8],
            default_aspect_ratio: Some(AspectRatioV2::Ratio16x9),
            default_resolution: None,
            default_duration: Some(8),
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/ai-models/veo3.svg".to_string()),
            ios_model_icon: Some("https://yral.com/img/ai-models/veo3.png".to_string()),
            extra_info: HashMap::new(),
        }
    }
}
