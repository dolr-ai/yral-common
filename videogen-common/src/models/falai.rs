use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, ModelMetadata, Veo3AspectRatio, VideoGenProvider, VideoGenerator};
use crate::video_model::VideoModel;
use crate::VideoGenError;
use candid::CandidType;
use global_constants::SEEDANCE_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct FalAiModel {
    pub prompt: String,
    pub model: String,
    pub seed: Option<u64>,
    pub num_frames: Option<u32>,
}

impl VideoGenerator for FalAiModel {
    fn model_id(&self) -> &'static str {
        &Self::model_info().id
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

    fn get_image(&self) -> Option<&ImageData> {
        None // FalAi doesn't support image input yet
    }

    fn get_image_mut(&mut self) -> Option<&mut ImageData> {
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

static FALAI_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
    id: "doubao-seedance-1-0-pro".to_string(),
    name: "Seedance 1.0 Pro".to_string(),
    description: "From Tiktok".to_string(),
    cost_usd_cents: SEEDANCE_COST_USD_CENTS,
    supports_image: true,
    provider: VideoGenProvider::FalAi,
    max_duration_seconds: 8,
    supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
    model_icon: Some("/img/ai-models/bytedance.svg".to_string()),
    is_available: false,
});

impl ModelMetadata for FalAiModel {
    fn model_info() -> &'static VideoModel {
        &FALAI_MODEL_INFO
    }
}
