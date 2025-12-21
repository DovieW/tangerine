//! Ollama LLM provider for local text formatting.

use super::{LlmError, LlmProvider};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.2";
/// Longer timeout for local models which may be slower
const DEFAULT_OLLAMA_TIMEOUT: Duration = Duration::from_secs(120);

/// Ollama LLM provider for local inference
pub struct OllamaLlmProvider {
    client: Client,
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaLlmProvider {
    /// Create a new Ollama provider with default settings
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            timeout: DEFAULT_OLLAMA_TIMEOUT,
        }
    }

    /// Create with a specific model
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_model(model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            model,
            timeout: DEFAULT_OLLAMA_TIMEOUT,
        }
    }

    /// Create with custom URL and model
    pub fn with_url(base_url: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: DEFAULT_OLLAMA_TIMEOUT,
        }
    }

    /// Create with custom client and settings
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_client(client: Client, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client,
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: DEFAULT_OLLAMA_TIMEOUT,
        }
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if Ollama is available at the configured URL
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .is_ok()
    }

    /// List available models
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| LlmError::ProviderNotAvailable(format!("Ollama not reachable: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::ProviderNotAvailable(
                "Failed to list Ollama models".to_string(),
            ));
        }

        let tags_response: TagsResponse = response.json().await.map_err(|e| {
            LlmError::InvalidResponse(format!("Failed to parse tags response: {}", e))
        })?;

        Ok(tags_response.models.into_iter().map(|m| m.name).collect())
    }
}

impl Default for OllamaLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    options: Option<ChatOptions>,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[async_trait]
impl LlmProvider for OllamaLlmProvider {
    async fn complete(&self, system_prompt: &str, user_message: &str) -> Result<String, LlmError> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            stream: false,
            options: Some(ChatOptions {
                temperature: 0.3,
                num_predict: 4096,
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(self.timeout)
                } else if e.is_connect() {
                    LlmError::ProviderNotAvailable(format!(
                        "Ollama not reachable at {}: {}",
                        self.base_url, e
                    ))
                } else {
                    LlmError::Network(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Try to parse as error response
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(LlmError::Api(format!(
                    "Ollama error ({}): {}",
                    status, error_response.error
                )));
            }
            return Err(LlmError::Api(format!(
                "Ollama error ({}): {}",
                status, error_text
            )));
        }

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            LlmError::InvalidResponse(format!("Failed to parse response: {}", e))
        })?;

        Ok(chat_response.message.content)
    }

    fn name(&self) -> &'static str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = OllamaLlmProvider::new();
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_default_model() {
        let provider = OllamaLlmProvider::new();
        assert_eq!(provider.model(), DEFAULT_MODEL);
    }

    #[test]
    fn test_custom_model() {
        let provider = OllamaLlmProvider::with_model("mistral".to_string());
        assert_eq!(provider.model(), "mistral");
    }

    #[test]
    fn test_custom_url() {
        let provider = OllamaLlmProvider::with_url(
            "http://192.168.1.100:11434".to_string(),
            Some("codellama".to_string()),
        );
        assert_eq!(provider.base_url, "http://192.168.1.100:11434");
        assert_eq!(provider.model(), "codellama");
    }
}
