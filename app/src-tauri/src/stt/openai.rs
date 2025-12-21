//! OpenAI STT provider implementation.
//!
//! Supports two modes:
//! - Legacy Whisper API (whisper-1) - uses /v1/audio/transcriptions
//! - GPT-4o Audio Preview - uses /v1/chat/completions with audio input

use super::{AudioFormat, SttError, SttProvider};
use async_trait::async_trait;
use reqwest::multipart;
use serde_json::json;
use std::time::Duration;

/// OpenAI STT provider for speech-to-text
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
    /// * `model` - Model to use:
    ///   - "gpt-4o-audio-preview" (default) - GPT-4o with audio input
    ///   - "gpt-4o-mini-audio-preview" - Smaller/faster GPT-4o audio
    ///   - "whisper-1" - Legacy Whisper API
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120)) // Longer timeout for GPT-4o
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o-audio-preview".to_string()),
        }
    }

    /// Create a new provider with a custom HTTP client
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_client(client: reqwest::Client, api_key: String, model: Option<String>) -> Self {
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o-audio-preview".to_string()),
        }
    }

    /// Check if using GPT-4o audio model
    fn is_gpt4o_audio(&self) -> bool {
        self.model.contains("gpt-4o") && self.model.contains("audio")
    }

    /// Transcribe using the legacy Whisper API
    async fn transcribe_whisper(&self, audio: &[u8]) -> Result<String, SttError> {
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
            .await
            .map_err(|e| if e.is_timeout() { SttError::Timeout } else { SttError::Network(e) })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SttError::Api(format!(
                "OpenAI Whisper API error ({}): {}",
                status, error_text
            )));
        }

        let result: serde_json::Value = response.json().await?;
        let text = result["text"].as_str().unwrap_or("").to_string();

        Ok(text)
    }

    /// Transcribe using GPT-4o audio chat completions API
    async fn transcribe_gpt4o(&self, audio: &[u8]) -> Result<String, SttError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Encode audio as base64
        let audio_base64 = STANDARD.encode(audio);

        let request_body = json!({
            "model": self.model,
            "modalities": ["text"],
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_audio",
                            "input_audio": {
                                "data": audio_base64,
                                "format": "wav"
                            }
                        },
                        {
                            "type": "text",
                            "text": "Transcribe this audio. Output only the transcribed text, nothing else."
                        }
                    ]
                }
            ]
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request_body)
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
                "OpenAI GPT-4o API error ({}): {}",
                status, error_text
            )));
        }

        let result: serde_json::Value = response.json().await?;

        // Extract text from chat completion response
        let text = result["choices"]
            .get(0)
            .and_then(|c| c["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

#[async_trait]
impl SttProvider for OpenAiSttProvider {
    async fn transcribe(&self, audio: &[u8], _format: &AudioFormat) -> Result<String, SttError> {
        if self.is_gpt4o_audio() {
            self.transcribe_gpt4o(audio).await
        } else {
            self.transcribe_whisper(audio).await
        }
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
        assert_eq!(provider.model, "gpt-4o-audio-preview");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider =
            OpenAiSttProvider::new("test-key".to_string(), Some("whisper-1".to_string()));
        assert_eq!(provider.model, "whisper-1");
    }

    #[test]
    fn test_is_gpt4o_audio() {
        let provider = OpenAiSttProvider::new("test-key".to_string(), None);
        assert!(provider.is_gpt4o_audio());

        let provider = OpenAiSttProvider::new(
            "test-key".to_string(),
            Some("gpt-4o-mini-audio-preview".to_string()),
        );
        assert!(provider.is_gpt4o_audio());

        let provider =
            OpenAiSttProvider::new("test-key".to_string(), Some("whisper-1".to_string()));
        assert!(!provider.is_gpt4o_audio());
    }
}
