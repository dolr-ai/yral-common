use crate::generator::FlowControlFromEnv;
use crate::types::{ImageInput, VideoGenProvider, VideoGenerator};
use crate::VideoGenError;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct FalAiModel {
    pub prompt: String,
    pub model: String,
    pub seed: Option<u64>,
    pub num_frames: Option<u32>,
}

impl VideoGenerator for FalAiModel {
    fn model_name(&self) -> &'static str {
        "FALAI"
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::FalAi
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if self.model.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Model name cannot be empty".to_string(),
            ));
        }

        if let Some(num_frames) = self.num_frames {
            if num_frames == 0 || num_frames > 300 {
                return Err(VideoGenError::InvalidInput(
                    "Number of frames must be between 1 and 300".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn get_prompt(&self) -> &str {
        &self.prompt
    }

    fn get_image(&self) -> Option<&ImageInput> {
        None // FalAi doesn't support image input yet
    }

    fn flow_control_config(&self) -> Option<(u32, u32)> {
        // No flow control for test model
        None
    }
}

impl FlowControlFromEnv for FalAiModel {
    fn env_prefix(&self) -> &'static str {
        "FALAI"
    }
}
