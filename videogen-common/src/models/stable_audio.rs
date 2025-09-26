use crate::generator::FlowControlFromEnv;
use crate::types::{VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct StableAudioModel {
    pub prompt: String,
    pub duration: u32, // Duration in seconds (default: 90)
}

impl Default for StableAudioModel {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            duration: 90,
        }
    }
}

impl VideoGenerator for StableAudioModel {
    fn model_id(&self) -> &'static str {
        "stable_audio"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::StableAudio
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Audio prompt cannot be empty".to_string(),
            ));
        }

        // Validate duration is within reasonable limits
        if self.duration < 1 || self.duration > 180 {
            return Err(VideoGenError::InvalidInput(
                "Duration must be between 1 and 180 seconds".to_string(),
            ));
        }

        Ok(())
    }

    fn get_prompt(&self) -> &str {
        &self.prompt
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

impl FlowControlFromEnv for StableAudioModel {
    fn env_prefix(&self) -> &'static str {
        "STABLE_AUDIO"
    }
}

impl StableAudioModel {
    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        // Stable Audio doesn't support image input
        if unified.image.is_some() {
            return Err(VideoGenError::InvalidInput(
                "Stable Audio does not support image input".to_string(),
            ));
        }

        // Validate prompt not empty
        if unified.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Extract duration from unified request or use default
        let duration = unified.duration_seconds.map(|d| d as u32).unwrap_or(90);

        // Check for duration in extra_params
        let duration = if let Some(duration_value) = unified.extra_params.get("duration") {
            duration_value.as_u64().unwrap_or(duration as u64) as u32
        } else {
            duration
        };

        Ok(VideoGenInput::StableAudio(StableAudioModel {
            prompt: unified.prompt,
            duration,
        }))
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::{AspectRatioV2, CostInfo, ResolutionV2};
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "stable_audio".to_string(),
            name: "Stable Audio 2.5".to_string(),
            description: "Stability AI's advanced audio generation model for creating high-quality sound effects and music".to_string(),
            cost: CostInfo::from_usd_cents(5), // Estimated cost per generation
            supports_image: false,
            supports_negative_prompt: false,
            supports_audio: true, // This model generates audio
            supports_audio_input: false,
            supports_seed: false,
            allowed_aspect_ratios: vec![],
            allowed_resolutions: vec![],
            allowed_durations: vec![5, 10, 15, 30, 60, 90, 120, 180],
            default_aspect_ratio: None,
            default_resolution: None,
            default_duration: Some(90),
            is_available: true,
            is_internal: false,
            model_icon: Some("https://replicate.com/stability-ai.png".to_string()),
            ios_model_icon: Some("https://replicate.com/stability-ai.png".to_string()),
            extra_info: HashMap::from([
                ("type".to_string(), serde_json::json!("audio_generator")),
                ("provider".to_string(), serde_json::json!("replicate")),
            ]),
        }
    }
}