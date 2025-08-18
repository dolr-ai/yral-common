#[cfg(feature = "client")]
use crate::types::VideoGenRequestKey;
use crate::types::{
    VideoGenError, VideoGenInput, VideoGenRequest, VideoGenRequestWithIdentity,
    VideoGenRequestWithSignature,
};
#[cfg(feature = "client")]
use crate::VideoGenRequestStatus;
use candid::Principal;
use reqwest::Url;

#[cfg(feature = "client")]
use yral_canisters_client::rate_limits::RateLimits;

pub struct VideoGenClient {
    base_url: Url,
    client: reqwest::Client,
    bearer_token: Option<String>,
}

impl VideoGenClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            bearer_token: None,
        }
    }

    pub fn with_bearer_token(base_url: Url, bearer_token: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            bearer_token: Some(bearer_token),
        }
    }

    /// Generate a video with the given request (returns queued response)
    pub async fn generate(
        &self,
        request: VideoGenRequest,
    ) -> Result<crate::types::VideoGenQueuedResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v1/videogen/generate")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let mut req_builder = self.client.post(url).json(&request);

        // Add bearer token if available
        if let Some(token) = &self.bearer_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error: VideoGenError = response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;
            Err(error)
        }
    }

    /// Generate a video with just the input (convenience method for backward compatibility)
    pub async fn generate_with_input(
        &self,
        principal: Principal,
        input: VideoGenInput,
    ) -> Result<crate::types::VideoGenQueuedResponse, VideoGenError> {
        let request = VideoGenRequest {
            principal,
            input,
            token_type: Default::default(),
        };
        self.generate(request).await
    }

    /// Generate a video with a signed request (returns queued response)
    pub async fn generate_with_signature(
        &self,
        signed_request: VideoGenRequestWithSignature,
    ) -> Result<crate::types::VideoGenQueuedResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v1/videogen/generate_signed")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let req_builder = self.client.post(url).json(&signed_request);

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error text".to_string());

            // Try to parse as VideoGenError
            match serde_json::from_str::<VideoGenError>(&error_text) {
                Ok(error) => Err(error),
                Err(_) => Err(VideoGenError::NetworkError(format!(
                    "Server error: {error_text}"
                ))),
            }
        }
    }

    /// Generate a video with delegated identity (returns queued response)
    pub async fn generate_with_identity(
        &self,
        identity_request: VideoGenRequestWithIdentity,
    ) -> Result<crate::types::VideoGenQueuedResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v1/videogen/generate_with_identity")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let req_builder = self.client.post(url).json(&identity_request);

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error text".to_string());

            // Try to parse as VideoGenError
            match serde_json::from_str::<VideoGenError>(&error_text) {
                Ok(error) => Err(error),
                Err(_) => Err(VideoGenError::NetworkError(format!(
                    "Server error: {error_text}"
                ))),
            }
        }
    }

    /// Get all available providers (V2 API)
    pub async fn get_providers(&self) -> Result<crate::types_v2::ProvidersResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v2/videogen/providers")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let mut req_builder = self.client.get(url);

        // Add bearer token if available
        if let Some(token) = &self.bearer_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error text".to_string());

            Err(VideoGenError::NetworkError(format!(
                "Failed to get providers: {error_text}"
            )))
        }
    }

    /// Get all providers including internal/test models (V2 API)
    pub async fn get_providers_all(
        &self,
    ) -> Result<crate::types_v2::ProvidersResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v2/videogen/providers-all")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let mut req_builder = self.client.get(url);

        // Add bearer token if available
        if let Some(token) = &self.bearer_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error text".to_string());

            Err(VideoGenError::NetworkError(format!(
                "Failed to get all providers: {error_text}"
            )))
        }
    }

    /// Generate a video with unified request structure (V2 API)
    pub async fn generate_with_identity_v2(
        &self,
        identity_request: crate::types_v2::VideoGenRequestWithIdentityV2,
    ) -> Result<crate::types_v2::VideoGenQueuedResponseV2, VideoGenError> {
        let url = self
            .base_url
            .join("api/v2/videogen/generate")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {e}")))?;

        let req_builder = self.client.post(url).json(&identity_request);

        let response = req_builder
            .send()
            .await
            .map_err(|e| VideoGenError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| VideoGenError::NetworkError(e.to_string()))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error text".to_string());

            // Try to parse as VideoGenError
            match serde_json::from_str::<VideoGenError>(&error_text) {
                Ok(error) => Err(error),
                Err(_) => Err(VideoGenError::NetworkError(format!(
                    "Server error: {error_text}"
                ))),
            }
        }
    }

    /// Poll the video generation status using a pre-configured RateLimits client
    #[cfg(feature = "client")]
    pub async fn poll_video_status_with_client(
        &self,
        request_key: &VideoGenRequestKey,
        rate_limits_client: &RateLimits<'_>,
    ) -> Result<VideoGenRequestStatus, VideoGenError> {
        let canister_request_key = yral_canisters_client::rate_limits::VideoGenRequestKey {
            principal: request_key.principal,
            counter: request_key.counter,
        };

        match rate_limits_client
            .poll_video_generation_status(canister_request_key)
            .await
        {
            Ok(result) => match result {
                yral_canisters_client::rate_limits::Result2::Ok(status) => Ok(status),
                yral_canisters_client::rate_limits::Result2::Err(err) => Err(
                    VideoGenError::NetworkError(format!("Rate limit error: {err}")),
                ),
            },
            Err(e) => Err(VideoGenError::NetworkError(format!(
                "Failed to poll status from canister: {e}"
            ))),
        }
    }
}
