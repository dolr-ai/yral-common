use crate::models::{FalAiModel, IntTestModel, LumaLabsModel, Veo3FastModel, Veo3Model};
use crate::types::{
    ImageInput, LumaLabsDuration, LumaLabsResolution, Veo3AspectRatio, VideoGenInput,
    VideoGenProvider,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema, CandidType)]
pub struct VideoModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost_sats: u64,
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
            cost_sats: 10,
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
            VideoModel {
                id: "veo3".to_string(),
                name: "Veo3".to_string(),
                description: "Google's advanced video generation model".to_string(),
                cost_sats: 0,
                supports_image: false,
                provider: VideoGenProvider::Veo3,
                max_duration_seconds: 8,
                supported_aspect_ratios: vec![
                    Veo3AspectRatio::Ratio16x9,
                    Veo3AspectRatio::Ratio9x16,
                ],
                model_icon: Some("/img/ai-models/veo3.svg".to_string()),
                is_available: true,
            },
            VideoModel {
                id: "veo3_fast".to_string(),
                name: "Veo3 Fast".to_string(),
                description: "Google Veo3 Faster and cheaper".to_string(),
                cost_sats: 0,
                supports_image: false,
                provider: VideoGenProvider::Veo3Fast,
                max_duration_seconds: 8,
                supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
                model_icon: Some("/img/ai-models/veo3.svg".to_string()),
                is_available: true,
            },
            VideoModel {
                id: "ray2flash".to_string(),
                name: "Ray2Flash".to_string(),
                description: "LumaLabs' fast AI video generation model".to_string(),
                cost_sats: 20,
                supports_image: true,
                provider: VideoGenProvider::LumaLabs,
                max_duration_seconds: 9,
                supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
                model_icon: Some("/img/ai-models/lumalabs.png".to_string()),
                is_available: true,
            },
            VideoModel {
                id: "doubao-seedance-1-0-pro".to_string(),
                name: "Seedance 1.0 Pro".to_string(),
                description: "From Tiktok".to_string(),
                cost_sats: 5,
                supports_image: true,
                provider: VideoGenProvider::FalAi,
                max_duration_seconds: 8,
                supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
                model_icon: Some("/img/ai-models/bytedance.svg".to_string()),
                is_available: false,
            },
            VideoModel {
                id: "inttest".to_string(),
                name: "IntTest".to_string(),
                description: "Test model that always returns the same video".to_string(),
                cost_sats: 0,
                supports_image: true,
                provider: VideoGenProvider::IntTest,
                max_duration_seconds: 5,
                supported_aspect_ratios: vec![Veo3AspectRatio::Ratio16x9],
                model_icon: Some("/img/yral/favicon.svg".to_string()),
                is_available: true,
            },
        ]
    }

    /// Convert this model to a VideoGenInput with the given prompt and optional image
    pub fn to_video_gen_input(
        &self,
        prompt: String,
        image: Option<ImageInput>,
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
            VideoGenProvider::FalAi => Ok(VideoGenInput::FalAi(FalAiModel {
                prompt,
                model: self.id.clone(),
                seed: None,
                num_frames: Some((self.max_duration_seconds as u32) * 30), // 30 fps
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
