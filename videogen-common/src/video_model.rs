use crate::models::IntTestModel;
use crate::types::{ImageData, Veo3AspectRatio, VideoGenInput, VideoGenProvider};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// TODO: Deprecated. Remove when mweb shifts to v2 APIs

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema, CandidType)]
pub struct VideoModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost_usd_cents: u64,
    pub supports_image: bool,
    pub provider: VideoGenProvider,
    pub max_duration_seconds: u8,
    pub supported_aspect_ratios: Vec<Veo3AspectRatio>,
    #[schema(example = "/img/ai-models/veo3.svg")]
    pub model_icon: Option<String>, // Path to icon in public folder
    pub is_available: bool, // Whether the model is currently available or coming soon
}

impl Default for VideoModel {
    fn default() -> Self {
        Self {
            id: "wan2_5_fast".to_string(),
            name: "Wan 2.5 Fast".to_string(),
            description: "Alibaba's Wan 2.5 Fast model for quick 720p video generation".to_string(),
            cost_usd_cents: 10,
            supports_image: true,
            provider: VideoGenProvider::Wan25Fast,
            max_duration_seconds: 5,
            supported_aspect_ratios: vec![Veo3AspectRatio::Ratio9x16],
            model_icon: None,
            is_available: true,
        }
    }
}

impl VideoModel {
    /// Get all available video generation models
    pub fn get_models() -> Vec<Self> {
        vec![
            Self::default(), // Wan25Fast
            IntTestModel::model_info().clone(),
        ]
    }

    /// Convert this model to a VideoGenInput with the given prompt and optional image
    pub fn to_video_gen_input(
        &self,
        prompt: String,
        image: Option<ImageData>,
    ) -> Result<VideoGenInput, String> {
        // Check if model is available
        if !self.is_available {
            return Err(format!("Model {} is coming soon", self.name));
        }

        // Check if image is provided but model doesn't support it
        if image.is_some() && !self.supports_image {
            return Err(format!("Model {} does not support image input", self.name));
        }

        match self.provider {
            VideoGenProvider::IntTest => Ok(VideoGenInput::IntTest(IntTestModel { prompt, image })),
            _ => Err(format!("Model {} is not supported", self.name)),
        }
    }

    /// Get the display duration string for UI
    pub fn duration_display(&self) -> String {
        if self.max_duration_seconds < 60 {
            format!("{} Sec", self.max_duration_seconds)
        } else {
            format!("{} Min", self.max_duration_seconds / 60)
        }
    }
}
