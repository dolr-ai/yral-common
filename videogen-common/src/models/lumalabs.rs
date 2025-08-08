use crate::generator::FlowControlFromEnv;
use crate::types::{
    ImageData, LumaLabsDuration, LumaLabsResolution, ModelMetadata, Veo3AspectRatio,
    VideoGenProvider, VideoGenerator,
};
use crate::video_model::VideoModel;
use crate::VideoGenError;
use candid::CandidType;
use global_constants::RAY2FLASH_COST_USD_CENTS;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, CandidType)]
pub struct LumaLabsModel {
    pub prompt: String,
    pub image: Option<ImageData>,
    pub resolution: LumaLabsResolution,
    pub duration: LumaLabsDuration,
    pub aspect_ratio: Option<String>,
    pub loop_video: bool,
}

impl VideoGenerator for LumaLabsModel {
    fn model_id(&self) -> &'static str {
        &Self::model_info().id
    }

    fn provider(&self) -> VideoGenProvider {
        VideoGenProvider::LumaLabs
    }

    fn validate_input(&self) -> Result<(), VideoGenError> {
        if self.prompt.is_empty() {
            return Err(VideoGenError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Validate aspect ratio format if provided
        if let Some(aspect_ratio) = &self.aspect_ratio {
            if !aspect_ratio.contains(':') {
                return Err(VideoGenError::InvalidInput(
                    "Aspect ratio must be in format 'width:height'".to_string(),
                ));
            }
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

impl FlowControlFromEnv for LumaLabsModel {
    fn env_prefix(&self) -> &'static str {
        "LUMALABS"
    }
}

static LUMALABS_MODEL_INFO: LazyLock<VideoModel> = LazyLock::new(|| VideoModel {
    id: "ray2flash".to_string(),
    name: "Ray2Flash".to_string(),
    description: "LumaLabs' fast AI video generation model".to_string(),
    cost_usd_cents: RAY2FLASH_COST_USD_CENTS,
    supports_image: true,
    provider: VideoGenProvider::LumaLabs,
    max_duration_seconds: 9,
    supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
    model_icon: Some("/img/ai-models/lumalabs.png".to_string()),
    is_available: true,
});

impl ModelMetadata for LumaLabsModel {
    fn model_info() -> &'static VideoModel {
        &LUMALABS_MODEL_INFO
    }
}
