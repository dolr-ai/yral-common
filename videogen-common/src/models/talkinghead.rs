use crate::generator::FlowControlFromEnv;
use crate::types::{AudioData, ImageData, VideoGenProvider, VideoGenerator};
use crate::{VideoGenError, VideoGenInput};
use candid::CandidType;
use global_constants::TALKINGHEAD_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct TalkingHeadModel {
    /// Input image (face) for the talking head
    pub image: ImageData,
    /// Input audio (voice) for the talking head
    pub audio: AudioData,
    // Note: No prompt field - audio serves as the input
    pub prompt: String, // Placeholder for backend validation
}

impl VideoGenerator for TalkingHeadModel {
    fn model_id(&self) -> &'static str {
        "talkinghead"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::TalkingHead
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        // Both image and audio are required
        // Validation is implicit in the struct fields being non-Option
        Ok(())
    }

    fn get_prompt(&self) -> &str {
        // TalkingHead doesn't use text prompts
        &self.prompt
    }

    fn get_image(&self) -> Option<&ImageData> {
        Some(&self.image)
    }

    fn get_image_mut(&mut self) -> Option<&mut ImageData> {
        Some(&mut self.image)
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        // No flow control for talking head model
        None
    }
}

impl FlowControlFromEnv for TalkingHeadModel {
    fn env_prefix(&self) -> &'static str {
        "TALKINGHEAD"
    }
}

impl TalkingHeadModel {
    /// Get the audio data reference
    pub fn get_audio(&self) -> &AudioData {
        &self.audio
    }

    /// Get mutable audio data reference
    pub fn get_audio_mut(&mut self) -> &mut AudioData {
        &mut self.audio
    }

    /// Create from unified v2 request
    pub fn from_unified_request(
        unified: crate::types_v2::VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        // Validate before creating
        Self::validate_unified_parameters(&unified)?;

        // Extract required fields
        let image = unified.image.ok_or_else(|| {
            VideoGenError::InvalidInput("TalkingHead requires an image input".to_string())
        })?;

        let audio = unified.audio.ok_or_else(|| {
            VideoGenError::InvalidInput("TalkingHead requires an audio input".to_string())
        })?;

        Ok(VideoGenInput::TalkingHead(TalkingHeadModel {
            image,
            audio,
            prompt: "[TalkingHead: Audio-based generation]".to_string(), // Placeholder for backend validation
        }))
    }

    /// Validate parameters from unified request
    pub fn validate_unified_parameters(
        unified: &crate::types_v2::VideoGenRequestV2,
    ) -> Result<(), VideoGenError> {
        // TalkingHead requires image
        if unified.image.is_none() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead requires an image (face) input".to_string(),
            ));
        }

        // TalkingHead requires audio
        if unified.audio.is_none() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead requires an audio (voice) input".to_string(),
            ));
        }

        // TalkingHead doesn't use text prompts, but allow placeholder for backend validation
        if !unified.prompt.is_empty() && unified.prompt != "[TalkingHead: Audio-based generation]" {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead does not use text prompts. Audio serves as the input.".to_string(),
            ));
        }

        // TalkingHead doesn't support negative prompts
        if unified.negative_prompt.is_some() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead does not support negative prompts".to_string(),
            ));
        }

        // TalkingHead doesn't support seed
        if unified.seed.is_some() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead does not support seed parameter".to_string(),
            ));
        }

        // TalkingHead doesn't support resolution parameter
        if unified.resolution.is_some() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead does not support resolution parameter".to_string(),
            ));
        }

        // TalkingHead doesn't support aspect ratio parameter
        if unified.aspect_ratio.is_some() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead does not support aspect ratio parameter".to_string(),
            ));
        }

        // TalkingHead doesn't support duration parameter
        if unified.duration_seconds.is_some() {
            return Err(VideoGenError::InvalidInput(
                "TalkingHead duration is determined by audio length".to_string(),
            ));
        }

        Ok(())
    }

    /// Get provider information for v2 API
    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        use crate::types_v2::CostInfo;
        use std::collections::HashMap;

        crate::types_v2::ProviderInfo {
            id: "talkinghead".to_string(),
            name: "Talking Head".to_string(),
            description: "Generate realistic talking head videos from image and audio".to_string(),
            cost: CostInfo::from_usd_cents(TALKINGHEAD_COST_USD_CENTS),
            supports_image: true, // Requires image input
            supports_negative_prompt: false,
            supports_audio: true, // Generates audio (talking voice) in the output video
            supports_audio_input: true, // Requires audio input
            supports_seed: false,
            allowed_aspect_ratios: vec![], // Determined by input image
            allowed_resolutions: vec![],   // Determined by input image
            allowed_durations: vec![],     // Determined by audio length
            default_aspect_ratio: None,
            default_resolution: None,
            default_duration: None, // Duration based on audio
            is_available: true,
            is_internal: false,
            model_icon: Some("https://yral.com/img/yral/favicon.svg".to_string()),
            extra_info: HashMap::new(),
        }
    }
}
