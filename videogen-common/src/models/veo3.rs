use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, ModelMetadata, Veo3AspectRatio, VideoGenProvider, VideoGenerator};
use crate::video_model::VideoModel;
use crate::VideoGenError;
use candid::CandidType;
use global_constants::VEO3_COST_USD_CENTS;
use global_constants::VEO3_FAST_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
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
    fn model_id(&self) -> &'static str {
        &Self::model_info().id
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

static VEO3_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
    id: "veo3".to_string(),
    name: "Veo3".to_string(),
    description: "Google's advanced video generation model".to_string(),
    cost_usd_cents: VEO3_COST_USD_CENTS,
    supports_image: false,
    provider: VideoGenProvider::Veo3,
    max_duration_seconds: 8,
    supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9, Veo3AspectRatio::Ratio9x16],
    model_icon: Some("/img/ai-models/veo3.svg".to_string()),
    is_available: true,
});

impl ModelMetadata for Veo3Model {
    fn model_info() -> &'static VideoModel {
        &VEO3_MODEL_INFO
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
    fn model_id(&self) -> &'static str {
        &Self::model_info().id
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

static VEO3_FAST_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
    id: "veo3_fast".to_string(),
    name: "Veo3 Fast".to_string(),
    description: "Google Veo3 Faster and cheaper".to_string(),
    cost_usd_cents: VEO3_FAST_COST_USD_CENTS,
    supports_image: false,
    provider: VideoGenProvider::Veo3Fast,
    max_duration_seconds: 8,
    supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
    model_icon: Some("/img/ai-models/veo3.svg".to_string()),
    is_available: true,
});

impl ModelMetadata for Veo3FastModel {
    fn model_info() -> &'static VideoModel {
        &VEO3_FAST_MODEL_INFO
    }
}
