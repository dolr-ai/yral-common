use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, Veo3AspectRatio, VideoGenProvider, VideoGenerator};
use crate::VideoGenError;
use candid::CandidType;
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
    fn model_name(&self) -> &'static str {
        "VEO3"
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
    fn model_name(&self) -> &'static str {
        "VEO3_FAST"
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
