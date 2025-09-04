use crate::models::{IntTestModel, LumaLabsModel, Veo3FastModel, Veo3Model};
use crate::types::{
    ImageData, LumaLabsDuration, LumaLabsResolution, ModelMetadata, Veo3AspectRatio, VideoGenInput,
    VideoGenProvider,
};
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
            id: "veo3".to_string(),
            name: "Veo3".to_string(),
            description: "Google's advanced video generation model".to_string(),
            cost_usd_cents: 10,
            supports_image: true,
            provider: VideoGenProvider::Veo3,
            max_duration_seconds: 8,
            supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9, Veo3AspectRatio::Ratio9x16],
            model_icon: None,
            is_available: true,
        }
    }
}

impl VideoModel {
    /// Get all available video generation models
    pub fn get_models() -> Vec<Self> {
        vec![
            LumaLabsModel::model_info().clone(),
            Veo3FastModel::model_info().clone(),
            Veo3Model::model_info().clone(),
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
            VideoGenProvider::Veo3 => Ok(VideoGenInput::Veo3(Veo3Model {
                prompt,
                negative_prompt: None,
                image,
                aspect_ratio: self
                    .supported_aspect_ratios
                    .first()
                    .cloned()
                    .unwrap_or(Veo3AspectRatio::Ratio16x9),
                duration_seconds: self.max_duration_seconds,
                generate_audio: true,
            })),
            VideoGenProvider::Veo3Fast => Ok(VideoGenInput::Veo3Fast(Veo3FastModel {
                prompt,
                negative_prompt: None,
                image,
                aspect_ratio: self
                    .supported_aspect_ratios
                    .first()
                    .cloned()
                    .unwrap_or(Veo3AspectRatio::Ratio16x9),
                duration_seconds: self.max_duration_seconds,
                generate_audio: true,
            })),
            VideoGenProvider::LumaLabs => Ok(VideoGenInput::LumaLabs(LumaLabsModel {
                prompt,
                image,
                resolution: LumaLabsResolution::R1080p, // Default to 1080p
                duration: if self.max_duration_seconds <= 5 {
                    LumaLabsDuration::D5s
                } else {
                    LumaLabsDuration::D9s
                },
                aspect_ratio: Some("16:9".to_string()),
                loop_video: false,
            })),
            VideoGenProvider::IntTest => Ok(VideoGenInput::IntTest(IntTestModel { prompt, image })),
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
