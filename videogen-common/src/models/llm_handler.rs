use crate::generator::FlowControlFromEnv;
use crate::types::{VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct LlmHandlerModel {
    pub user_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmHandlerResponse {
    pub audio_description: String,
    pub video_description: String,
}

impl VideoGenerator for LlmHandlerModel {
    fn model_id(&self) -> &'static str {
        "llm_handler"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::LlmHandler
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.user_prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "User prompt cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn get_prompt(&self) -> &str {
        &self.user_prompt
    }

    fn get_image(&self) -> Option<&crate::types::ImageData> {
        None
    }

    fn get_image_mut(&mut self) -> Option<&mut crate::types::ImageData> {
        None
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        None
    }
}

impl FlowControlFromEnv for LlmHandlerModel {
    fn env_prefix(&self) -> &'static str {
        "LLM_HANDLER"
    }
}

impl LlmHandlerModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        // LLM handler doesn't support image input
        if unified.image.is_some() {
            return Err(VideoGenError::InvalidInput(
                "LLM Handler does not support image input".to_string(),
            ));
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        Ok(VideoGenInput::LlmHandler(LlmHandlerModel {
            user_prompt: unified.prompt,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "llm_handler".to_string(),
            name: "LLM Prompt Generator".to_string(),
            description: "Generates optimized audio and video descriptions from user prompts".to_string(),
            cost: CostInfo::from_usd_cents(1), // Minimal cost for LLM processing
            supports_image: false,
            supports_negative_prompt: false,
            supports_audio: false,
            supports_audio_input: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![],
            allowed_resolutions: vec![],
            allowed_durations: vec![],
            default_aspect_ratio: None,
            default_resolution: None,
            default_duration: None,
            is_available: true,
            is_internal: true, // Internal service
            model_icon: None,
            ios_model_icon: None,
            extra_info: HashMap::from([
                ("type".to_string(), serde_json::json!("prompt_generator")),
            ]),
        }
    }
}