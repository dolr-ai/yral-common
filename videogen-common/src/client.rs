use crate::types::{
    VideoGenError, VideoGenInput, VideoGenRequest, VideoGenRequestWithSignature,
};
#[cfg(feature = "client")]
use crate::types::VideoGenRequestKey;
#[cfg(feature = "client")]
use crate::VideoGenRequestStatus;
#[cfg(not(feature = "client"))]
use crate::types::{VideoGenRequestKey, VideoGenQueuedResponse};
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
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {}", e)))?;

        let mut req_builder = self.client.post(url).json(&request);

        // Add bearer token if available
        if let Some(token) = &self.bearer_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
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
        let request = VideoGenRequest { principal, input };
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
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {}", e)))?;

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
                    "Server error: {}",
                    error_text
                ))),
            }
        }
    }

    /// Poll the video generation status from the rate limits canister
    #[cfg(feature = "client")]
    pub async fn poll_video_status(
        &self,
        request_key: &VideoGenRequestKey,
        agent: &::ic_agent::Agent,
        rate_limits_canister_id: Principal,
    ) -> Result<VideoGenRequestStatus, VideoGenError> {
        let rate_limits_client = RateLimits(rate_limits_canister_id, agent);
        
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
                yral_canisters_client::rate_limits::Result2::Err(e) => {
                    Err(VideoGenError::NetworkError(format!("Rate limits error: {}", e)))
                }
            },
            Err(e) => Err(VideoGenError::NetworkError(format!(
                "Failed to poll status: {}",
                e
            ))),
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
                yral_canisters_client::rate_limits::Result2::Ok(status) => {
                    // Convert from canister VideoGenRequestStatus to our VideoGenRequestStatus
                    match status {
                        yral_canisters_client::rate_limits::VideoGenRequestStatus::Pending => {
                            Ok(VideoGenRequestStatus::Pending)
                        }
                        yral_canisters_client::rate_limits::VideoGenRequestStatus::Processing => {
                            Ok(VideoGenRequestStatus::Processing)
                        }
                        yral_canisters_client::rate_limits::VideoGenRequestStatus::Complete(url) => {
                            Ok(VideoGenRequestStatus::Complete(url))
                        }
                        yral_canisters_client::rate_limits::VideoGenRequestStatus::Failed(error) => {
                            Ok(VideoGenRequestStatus::Failed(error))
                        }
                    }
                }
                yral_canisters_client::rate_limits::Result2::Err(err) => {
                    Err(VideoGenError::NetworkError(format!("Rate limit error: {}", err)))
                }
            },
            Err(e) => Err(VideoGenError::NetworkError(format!(
                "Failed to poll status from canister: {}",
                e
            ))),
        }
    }
}
