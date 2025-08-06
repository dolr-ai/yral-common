use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, VideoGenProvider, VideoGenerator};
use crate::VideoGenError;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct IntTestModel {
    pub prompt: String,
    pub image: Option<ImageData>,
}

impl VideoGenerator for IntTestModel {
    fn model_name(&self) -> &'static str {
        "INTTEST"
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
