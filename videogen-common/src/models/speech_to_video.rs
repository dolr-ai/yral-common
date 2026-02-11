use std::collections::HashMap;

use candid::CandidType;
use global_constants::WAN2_5_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    types_v2::{AspectRatioV2, ResolutionV2},
    AudioData, CostInfo, VideoGenError, VideoGenInput, VideoGenRequestV2, VideoGenerator,
};

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct SpeechToVideoModel {
    /// Input audio (voice) for the speech-to-video model
    pub audio: AudioData,
    /// Optional image for image-to-video generation with speech
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<crate::types::ImageData>,
}

impl VideoGenerator for SpeechToVideoModel {
    fn model_id(&self) -> &'static str {
        "speech_to_video"
    }

    fn provider(&self) -> crate::types::VideoGenProvider {
        crate::types::VideoGenProvider::SpeechToVideo
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        // Audio is required
        // Validation is implicit in the struct fields being non-Option
        Ok(())
    }

    fn get_prompt(&self) -> &str {
        "" // No text prompt for speech-to-video
    }

    fn get_image(&self) -> Option<&crate::types::ImageData> {
        self.image.as_ref()
    }

    fn get_image_mut(&mut self) -> Option<&mut crate::types::ImageData> {
        self.image.as_mut()
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        None // No flow control for speech-to-video model
    }

    #[doc = " Get flow control key for Qstash rate limiting"]
    fn flow_control_key(&self) -> String {
        format!("VIDEOGEN_{}", self.model_id())
    }

    fn supports_webhook_callbacks(&self) -> bool {
        true
    }
}

impl SpeechToVideoModel {
    pub fn from_unified_request(
        request: VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        if let Some(audio) = request.audio {
            Ok(VideoGenInput::SpeechToVideo(SpeechToVideoModel {
                audio,
                image: request.image,
            }))
        } else {
            Err(VideoGenError::InvalidInput(
                "Audio data is required for SpeechToVideoModel".to_string(),
            ))
        }
    }

    pub fn get_provider_info() -> crate::types_v2::ProviderInfo {
        crate::types_v2::ProviderInfo {
            id: "speech_to_video".to_string(),
            name: "SpeechToVideo".to_string(),
            description: "Generates videos from speech and optional image input using advanced AI models.".to_string(),
            supports_image: true,
            model_icon: Some("https://yral.com/img/yral/favicon.svg".to_string()),
            is_available: true,
            cost: CostInfo::from_usd_cents(WAN2_5_COST_USD_CENTS),
            supports_negative_prompt: false,
            supports_audio: true,
            supports_audio_input: true,
            supports_seed: false,
            allowed_aspect_ratios: vec![AspectRatioV2::Ratio9x16],
            allowed_resolutions: vec![ResolutionV2::R720p],
            default_aspect_ratio: Some(AspectRatioV2::Ratio9x16),
            default_resolution: Some(ResolutionV2::R720p),
            default_duration: None,
            is_internal: false,
            ios_model_icon: Some(
                "https://yral.com/img/yral/android-chrome-192x192.png".to_string(),
            ),
            extra_info: HashMap::from([
                (
                    "architecture".to_string(),
                    serde_json::json!("MoE Flow-Matching"),
                ),
                ("model_size".to_string(), serde_json::json!("14B")),
                ("speed".to_string(), serde_json::json!("optimized")),
            ]),
            allowed_durations: vec![5],
        }
    }
}
