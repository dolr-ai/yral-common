use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, VideoGenProvider, VideoGenerator};
// VideoModel and ModelMetadata have been removed
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::INTTEST_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct IntTestModel {
    pub prompt: String,
    pub image: Option<ImageData>,
}

impl VideoGenerator for IntTestModel {
    fn model_id(&self) -> &'static str {
        "inttest"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::IntTest
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

impl FlowControlFromEnv for IntTestModel {
    fn env_prefix(&self) -> &'static str {
        "INTTEST"
    }
}

// VideoModel has been removed - model info is now fetched dynamically via API
// static INTTEST_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
//     id: "inttest".to_string(),
//     name: "IntTest".to_string(),
//     description: "Test model that always returns the same video".to_string(),
//     cost_usd_cents: INTTEST_COST_USD_CENTS,
//     supports_image: true,
//     provider: VideoGenProvider::IntTest,
//     max_duration_seconds: 5,
//     supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
//     model_icon: Some("/img/yral/favicon.svg".to_string()),
//     is_available: true,
// });

// impl ModelMetadata for IntTestModel {
//     fn model_info() -> &'static VideoModel {
//         &INTTEST_MODEL_INFO
//     }
// }

impl IntTestModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        // Validate before creating
        Self::validate_unified_parameters(&unified)?;

        Ok(VideoGenInput::IntTest(IntTestModel {
            prompt: unified.prompt,
            image: unified.image,
        }))
    }

    /// Validate parameters from unified request
    pub fn validate_unified_parameters(
        unified: &crate::types_v2::VideoGenRequestV2,
    ) -> Result<(), VideoGenError> {
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "inttest".to_string(),
            name: "Internal Test".to_string(),
            description: "Internal test model for development".to_string(),
            cost: CostInfo::from_usd_cents(INTTEST_COST_USD_CENTS),
            supports_image: true,
            supports_negative_prompt: false,
            supports_audio: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio16x9],
            allowed_resolutions: vec![ResolutionV2::R1080p],
            allowed_durations: vec![5],
            default_aspect_ratio: Some(AspectRatioV2::Ratio16x9),
            default_resolution: Some(ResolutionV2::R1080p),
            default_duration: 5,
            is_available: true,
            is_internal: true,
            model_icon: Some("https://yral.com/img/yral/favicon.svg".to_string()),
            extra_info: HashMap::new(),
        }
    }
}
