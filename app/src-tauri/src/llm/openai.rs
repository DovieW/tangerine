//! OpenAI LLM provider for text formatting.

use super::{LlmError, LlmProvider, DEFAULT_LLM_TIMEOUT};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// OpenAI LLM provider using the Chat Completions API
pub struct OpenAiLlmProvider {
    client: Client,
    api_key: String,
    model: String,
    timeout: Duration,
}

impl OpenAiLlmProvider {
    /// Create a new OpenAI provider with the given API key
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: DEFAULT_MODEL.to_string(),
            timeout: DEFAULT_LLM_TIMEOUT,
        }
    }

    /// Create with a specific model
    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            timeout: DEFAULT_LLM_TIMEOUT,
        }
    }

    /// Create with custom client and settings
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_client(client: Client, api_key: String, model: Option<String>) -> Self {
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: DEFAULT_LLM_TIMEOUT,
        }
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn supports_structured_outputs(model: &str) -> bool {
        // GPT-4.1 family supports Structured Outputs; using it for rewrite makes outputs
        // deterministic and easier to parse.
        //
        // Docs: https://platform.openai.com/docs/guides/structured-outputs
        model.starts_with("gpt-4.1")
    }

    fn rewrite_response_format() -> ResponseFormat {
        // Keep the schema intentionally tiny: the app only needs the final rewritten text.
        // Rich field descriptions make the model's job (and prompt-writing) easier.
        ResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: JsonSchemaFormat {
                name: "rewrite_response".to_string(),
                strict: true,
                description: Some(
                    "Structured output for a dictation transcript rewrite. The model must emit valid JSON matching the schema."
                        .to_string(),
                ),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "rewritten_text": {
                            "type": "string",
                            "description": "The final rewritten transcript text. This string will be used directly as the output. Preserve meaning, intent, and any required formatting. Do not wrap in markdown or add extra commentary. Return an empty string only if the input transcript is empty."
                        }
                    },
                    "required": ["rewritten_text"],
                    "additionalProperties": false
                }),
            },
        }
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
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchemaFormat,
}

#[derive(Debug, Serialize)]
struct JsonSchemaFormat {
    name: String,
    strict: bool,
    schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
    #[serde(default)]
    refusal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
}

#[async_trait]
impl LlmProvider for OpenAiLlmProvider {
    async fn complete(&self, system_prompt: &str, user_message: &str) -> Result<String, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::NoApiKey("openai".to_string()));
        }

        let use_structured_outputs = Self::supports_structured_outputs(&self.model);

        // When using Structured Outputs, a short explicit instruction helps avoid
        // accidental prose even though the schema is enforced server-side.
        let system_prompt = if use_structured_outputs {
            format!(
                "{}\n\nReturn ONLY valid JSON that matches the provided JSON Schema (no markdown, no extra keys).",
                system_prompt
            )
        } else {
            system_prompt.to_string()
        };

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            max_tokens: 4096,
            temperature: 0.3, // Lower temperature for more consistent formatting
            response_format: use_structured_outputs
                .then(|| Self::rewrite_response_format()),
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .bearer_auth(&self.api_key)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(self.timeout)
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
                    "OpenAI API error ({}): {}",
                    status, error_response.error.message
                )));
            }
            return Err(LlmError::Api(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            LlmError::InvalidResponse(format!("Failed to parse response: {}", e))
        })?;

        let first = chat_response
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse("No response choices returned".to_string()))?;

        if let Some(refusal) = &first.message.refusal {
            return Err(LlmError::Api(format!("OpenAI refusal: {}", refusal)));
        }

        if use_structured_outputs {
            let v: serde_json::Value = serde_json::from_str(&first.message.content).map_err(|e| {
                LlmError::InvalidResponse(format!(
                    "Structured output was not valid JSON: {} (content: {})",
                    e, first.message.content
                ))
            })?;

            let rewritten = v
                .get("rewritten_text")
                .and_then(|t| t.as_str())
                .ok_or_else(|| {
                    LlmError::InvalidResponse(format!(
                        "Structured output missing required field 'rewritten_text' (content: {})",
                        first.message.content
                    ))
                })?;

            Ok(rewritten.to_string())
        } else {
            Ok(first.message.content.clone())
        }
    }

    fn name(&self) -> &'static str {
        "openai"
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
        let provider = OpenAiLlmProvider::new("test-key".to_string());
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_default_model() {
        let provider = OpenAiLlmProvider::new("test-key".to_string());
        assert_eq!(provider.model(), DEFAULT_MODEL);
    }

    #[test]
    fn test_custom_model() {
        let provider = OpenAiLlmProvider::with_model("test-key".to_string(), "gpt-4".to_string());
        assert_eq!(provider.model(), "gpt-4");
    }
}
