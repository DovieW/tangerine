//! OpenAI Whisper API STT provider implementation.

use super::{AudioFormat, SttError, SttProvider};
use async_trait::async_trait;
use reqwest::multipart;
use std::time::Duration;

/// OpenAI Whisper API provider for speech-to-text
pub struct OpenAiSttProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiSttProvider {
    /// Create a new OpenAI STT provider
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `model` - Model to use (e.g., "whisper-1")
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "whisper-1".to_string()),
        }
    }

    /// Create a new provider with a custom HTTP client
    pub fn with_client(client: reqwest::Client, api_key: String, model: Option<String>) -> Self {
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "whisper-1".to_string()),
        }
    }
}

#[async_trait]
impl SttProvider for OpenAiSttProvider {
    async fn transcribe(&self, audio: &[u8], _format: &AudioFormat) -> Result<String, SttError> {
        let part = multipart::Part::bytes(audio.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::Audio(format!("Failed to create multipart: {}", e)))?;

        let form = multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone());

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SttError::Api(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let result: serde_json::Value = response.json().await?;
        let text = result["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    fn name(&self) -> &'static str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAiSttProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.model, "whisper-1");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = OpenAiSttProvider::new("test-key".to_string(), Some("whisper-2".to_string()));
        assert_eq!(provider.model, "whisper-2");
    }
}
