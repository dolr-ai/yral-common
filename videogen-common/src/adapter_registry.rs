#[cfg(feature = "server")]
use std::sync::LazyLock;

use crate::models::speech_to_video::SpeechToVideoModel;
use crate::models::{IntTestModel, LumaLabsModel, TalkingHeadModel, Wan25FastModel, Wan25Model};
use crate::types::VideoGenError;
use crate::types_v2::{ProviderInfo, ProvidersResponse, VideoGenRequestV2};
use crate::VideoGenInput;

/// Registry for all available model adapters
pub struct AdapterRegistry;

impl AdapterRegistry {
    /// Adapt a unified request to model-specific format
    pub fn adapt_request(
        &self,
        request: VideoGenRequestV2,
    ) -> Result<VideoGenInput, VideoGenError> {
        match request.model_id.as_str() {
            "lumalabs" => LumaLabsModel::from_unified_request(request),
            "inttest" => IntTestModel::from_unified_request(request),
            "talkinghead" => TalkingHeadModel::from_unified_request(request),
            "wan2_5" => Wan25Model::from_unified_request(request),
            "wan2_5_fast" => Wan25FastModel::from_unified_request(request),
            "speech_to_video" => SpeechToVideoModel::from_unified_request(request),
            _ => Err(VideoGenError::InvalidInput(format!(
                "Unknown model: {}",
                request.model_id
            ))),
        }
    }

    /// Get provider information for all registered models
    pub fn get_all_providers(&self) -> ProvidersResponse {
        let providers = vec![
            Wan25FastModel::get_provider_info(), // Default
            Wan25Model::get_provider_info(),
            LumaLabsModel::get_provider_info(),
            IntTestModel::get_provider_info(),
        ];

        ProvidersResponse {
            providers,
            schema_version: "1.0.0".to_string(),
        }
    }

    /// Get provider information for all prod models
    pub fn get_all_prod_providers(&self) -> ProvidersResponse {
        let providers = vec![
            Wan25FastModel::get_provider_info(), // Default
            Wan25Model::get_provider_info(),
            LumaLabsModel::get_provider_info(),
        ];

        ProvidersResponse {
            providers,
            schema_version: "1.0.0".to_string(),
        }
    }

    /// Get provider information for a specific model
    pub fn get_provider_info(&self, model_id: &str) -> Option<ProviderInfo> {
        match model_id {
            "lumalabs" => Some(LumaLabsModel::get_provider_info()),
            "inttest" => Some(IntTestModel::get_provider_info()),
            "talkinghead" => Some(TalkingHeadModel::get_provider_info()),
            "wan2_5" => Some(Wan25Model::get_provider_info()),
            "wan2_5_fast" => Some(Wan25FastModel::get_provider_info()),
            _ => None,
        }
    }

    /// Check if a model is available
    pub fn is_model_available(&self, model_id: &str) -> bool {
        self.get_provider_info(model_id)
            .map(|info| info.is_available)
            .unwrap_or(false)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self
    }
}

/// Global adapter registry instance
#[cfg(feature = "server")]
pub static ADAPTER_REGISTRY: LazyLock<AdapterRegistry> = LazyLock::new(AdapterRegistry::default);
