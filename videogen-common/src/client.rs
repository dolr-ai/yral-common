use crate::types::{
    VideoGenError, VideoGenInput, VideoGenRequest, VideoGenRequestWithSignature, VideoGenResponse,
};
use candid::Principal;

pub struct VideoGenClient {
    base_url: String,
    client: reqwest::Client,
    bearer_token: Option<String>,
}

impl VideoGenClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            bearer_token: None,
        }
    }

    pub fn with_bearer_token(base_url: String, bearer_token: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            bearer_token: Some(bearer_token),
        }
    }

    /// Generate a video with the given request
    pub async fn generate(
        &self,
        request: VideoGenRequest,
    ) -> Result<VideoGenResponse, VideoGenError> {
        let mut req_builder = self
            .client
            .post(&format!("{}/api/v1/videogen/generate", self.base_url))
            .json(&request);

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
    ) -> Result<VideoGenResponse, VideoGenError> {
        let request = VideoGenRequest { principal, input };
        self.generate(request).await
    }

    /// Generate a video with a signed request
    pub async fn generate_with_signature(
        &self,
        signed_request: VideoGenRequestWithSignature,
    ) -> Result<VideoGenResponse, VideoGenError> {
        let mut req_builder = self
            .client
            .post(&format!(
                "{}/api/v1/videogen/generate_signed",
                self.base_url
            ))
            .json(&signed_request);

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
}
