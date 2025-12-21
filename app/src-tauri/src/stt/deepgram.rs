//! Deepgram STT provider implementation.

use super::{AudioFormat, SttError, SttProvider};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

/// Deepgram API provider for speech-to-text
pub struct DeepgramSttProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl DeepgramSttProvider {
    /// Create a new Deepgram STT provider
    ///
    /// # Arguments
    /// * `api_key` - Deepgram API key
    /// * `model` - Model to use (e.g., "nova-2")
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "nova-2".to_string()),
        }
    }

    /// Create a new provider with a custom HTTP client
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_client(client: reqwest::Client, api_key: String, model: Option<String>) -> Self {
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "nova-2".to_string()),
        }
    }
}

#[async_trait]
impl SttProvider for DeepgramSttProvider {
    async fn transcribe(&self, audio: &[u8], _format: &AudioFormat) -> Result<String, SttError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Token {}", self.api_key))
                .map_err(|e| SttError::Config(format!("Invalid API key format: {}", e)))?,
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("audio/wav"),
        );

        // Build URL with query parameters
        let url = format!(
            "https://api.deepgram.com/v1/listen?model={}&smart_format=true&punctuate=true",
            self.model
        );

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .body(audio.to_vec())
            .send()
            .await
            .map_err(|e| if e.is_timeout() { SttError::Timeout } else { SttError::Network(e) })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SttError::Api(format!(
                "Deepgram API error ({}): {}",
                status, error_text
            )));
        }

        let result: serde_json::Value = response.json().await?;

        // Deepgram response structure:
        // { "results": { "channels": [{ "alternatives": [{ "transcript": "..." }] }] } }
        let text = result["results"]["channels"]
            .get(0)
            .and_then(|ch| ch["alternatives"].get(0))
            .and_then(|alt| alt["transcript"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    fn name(&self) -> &'static str {
        "deepgram"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = DeepgramSttProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "deepgram");
        assert_eq!(provider.model, "nova-2");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = DeepgramSttProvider::new("test-key".to_string(), Some("nova-2-general".to_string()));
        assert_eq!(provider.model, "nova-2-general");
    }
}
