use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{AudioData, VideoGenError, VideoGenInput, VideoGenRequestV2, VideoGenerator};

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct SpeechToVideoModel {
    /// Input audio (voice) for the speech-to-video model
    pub audio: AudioData,
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
        None // No image support
    }

    fn get_image_mut(&mut self) -> Option<&mut crate::types::ImageData> {
        None
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
            Ok(VideoGenInput::SpeechToVideo(SpeechToVideoModel { audio }))
        } else {
            Err(VideoGenError::InvalidInput(
                "Audio data is required for SpeechToVideoModel".to_string(),
            ))
        }
    }
}
