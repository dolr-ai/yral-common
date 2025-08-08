use crate::generator::FlowControlFromEnv;
use crate::types::{ImageData, ModelMetadata, Veo3AspectRatio, VideoGenProvider, VideoGenerator};
use crate::video_model::VideoModel;
use crate::VideoGenError;
use candid::CandidType;
use global_constants::INTTEST_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct IntTestModel {
    pub prompt: String,
    pub image: Option<ImageData>,
}

impl VideoGenerator for IntTestModel {
    fn model_id(&self) -> &'static str {
        &Self::model_info().id
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

static INTTEST_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
    id: "inttest".to_string(),
    name: "IntTest".to_string(),
    description: "Test model that always returns the same video".to_string(),
    cost_usd_cents: INTTEST_COST_USD_CENTS,
    supports_image: true,
    provider: VideoGenProvider::IntTest,
    max_duration_seconds: 5,
    supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
    model_icon: Some("/img/yral/favicon.svg".to_string()),
    is_available: true,
});

impl ModelMetadata for IntTestModel {
    fn model_info() -> &'static VideoModel {
        &INTTEST_MODEL_INFO
    }
}
