use crate::types::{
    VideoGenError, VideoGenInput, VideoGenRequest, VideoGenRequestWithSignature, VideoGenResponse,
};
use candid::Principal;
use reqwest::Url;

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

    /// Generate a video with the given request
    pub async fn generate(
        &self,
        request: VideoGenRequest,
    ) -> Result<VideoGenResponse, VideoGenError> {
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
    ) -> Result<VideoGenResponse, VideoGenError> {
        let request = VideoGenRequest { principal, input };
        self.generate(request).await
    }

    /// Generate a video with a signed request
    pub async fn generate_with_signature(
        &self,
        signed_request: VideoGenRequestWithSignature,
    ) -> Result<VideoGenResponse, VideoGenError> {
        let url = self
            .base_url
            .join("api/v1/videogen/generate_signed")
            .map_err(|e| VideoGenError::NetworkError(format!("Invalid URL: {}", e)))?;

        let req_builder = self.client.post(url).json(&signed_request);

        let response = req_builder.send().await.map_err(|e| {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(
                &format!("VideoGenClient: Network error during send: {}", e).into(),
            );
            #[cfg(not(target_arch = "wasm32"))]
            println!("VideoGenClient: Network error during send: {}", e);
            VideoGenError::NetworkError(e.to_string())
        })?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::error_1(
                    &format!("VideoGenClient: Error parsing success response: {}", e).into(),
                );
                #[cfg(not(target_arch = "wasm32"))]
                println!("VideoGenClient: Error parsing success response: {}", e);
                VideoGenError::NetworkError(e.to_string())
            })
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
}
