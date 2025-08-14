pub mod client;
pub mod generator;
pub mod models;
pub mod token_costs;
pub mod types;
pub mod video_model;

// V2 modules
pub mod adapter_registry;
pub mod types_v2;

pub use client::VideoGenClient;
pub use generator::FlowControlFromEnv;
pub use token_costs::{ModelCostUSD, TokenConversionRates, TokenCostConfig, TOKEN_COST_CONFIG};
pub use types::{
    ImageData, ImageInput, LumaLabsDuration, LumaLabsResolution, TokenType, Veo3AspectRatio,
    VideoGenError, VideoGenInput, VideoGenProvider, VideoGenQueuedResponse, VideoGenRequest,
    VideoGenRequestKey, VideoGenRequestWithIdentity, VideoGenRequestWithSignature,
    VideoGenResponse, VideoGenerator,
};
pub use video_model::VideoModel;

// V2 exports
pub use adapter_registry::{AdapterRegistry, ADAPTER_REGISTRY};
pub use types_v2::{
    CostInfo, ProviderInfo, ProvidersResponse, VideoGenQueuedResponseV2,
    VideoGenRequestV2, VideoGenRequestWithIdentityV2,
};

#[cfg(feature = "client")]
pub use yral_canisters_client::rate_limits::VideoGenRequestStatus;
