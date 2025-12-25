//! Groq Whisper API STT provider implementation.

use super::{AudioFormat, SttError, SttProvider};
use async_trait::async_trait;
use reqwest::multipart;
use std::time::Duration;

/// Groq Whisper API provider for speech-to-text
pub struct GroqSttProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    default_prompt: Option<String>,
}

impl GroqSttProvider {
    const PROMPT_MAX_CHARS: usize = 224;

    /// Create a new Groq STT provider
    ///
    /// # Arguments
    /// * `api_key` - Groq API key
    /// * `model` - Model to use (e.g., "whisper-large-v3-turbo")
    /// * `default_prompt` - Optional transcription prompt (OpenAI-compatible `prompt` field)
    pub fn new(api_key: String, model: Option<String>, default_prompt: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "whisper-large-v3-turbo".to_string()),
            default_prompt,
        }
    }

    /// Create a new provider with a custom HTTP client
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_client(
        client: reqwest::Client,
        api_key: String,
        model: Option<String>,
        default_prompt: Option<String>,
    ) -> Self {
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "whisper-large-v3-turbo".to_string()),
            default_prompt,
        }
    }

    fn clamp_prompt(prompt: &str) -> Option<String> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return None;
        }

        let clamped: String = trimmed.chars().take(Self::PROMPT_MAX_CHARS).collect();
        Some(clamped)
    }
}

#[async_trait]
impl SttProvider for GroqSttProvider {
    async fn transcribe(&self, audio: &[u8], _format: &AudioFormat) -> Result<String, SttError> {
        let part = multipart::Part::bytes(audio.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::Audio(format!("Failed to create multipart: {}", e)))?;

        let mut form = multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone());

        if let Some(prompt) = self
            .default_prompt
            .as_deref()
            .and_then(Self::clamp_prompt)
        {
            form = form.text("prompt", prompt);
        }

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
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
                "Groq API error ({}): {}",
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
        "groq"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = GroqSttProvider::new("test-key".to_string(), None, None);
        assert_eq!(provider.name(), "groq");
        assert_eq!(provider.model, "whisper-large-v3-turbo");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = GroqSttProvider::new(
            "test-key".to_string(),
            Some("whisper-large-v3-turbo".to_string()),
            None,
        );
        assert_eq!(provider.model, "whisper-large-v3-turbo");
    }

    #[test]
    fn test_prompt_clamping() {
        let long = "x".repeat(GroqSttProvider::PROMPT_MAX_CHARS + 10);
        let clamped = GroqSttProvider::clamp_prompt(&long).unwrap();
        assert_eq!(clamped.len(), GroqSttProvider::PROMPT_MAX_CHARS);
    }
}
