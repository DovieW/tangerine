//! Tauri commands for LLM formatting configuration.

use crate::llm::{
    LlmConfig, PromptSections, ADVANCED_PROMPT_DEFAULT, DICTIONARY_PROMPT_DEFAULT,
    MAIN_PROMPT_DEFAULT,
};
use crate::pipeline::SharedPipeline;
use std::time::Duration;
use tauri::State;

/// Error type for LLM commands
#[derive(Debug, serde::Serialize)]
pub struct LlmCommandError {
    pub message: String,
}

impl From<String> for LlmCommandError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

/// LLM configuration payload from frontend
#[derive(Debug, serde::Deserialize)]
pub struct LlmConfigPayload {
    /// Whether LLM formatting is enabled
    pub enabled: bool,
    /// Provider name: "openai", "anthropic", or "ollama"
    pub provider: String,
    /// API key (not needed for ollama)
    pub api_key: Option<String>,
    /// Model to use (optional, uses provider default if not specified)
    pub model: Option<String>,
    /// Base URL for Ollama (optional)
    pub ollama_url: Option<String>,
    /// Timeout in seconds (optional, default 30)
    pub timeout_secs: Option<u64>,
}

/// Prompt configuration payload from frontend
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PromptConfigPayload {
    /// Custom main prompt (null to use default)
    pub main_custom: Option<String>,
    /// Whether advanced section is enabled
    pub advanced_enabled: bool,
    /// Custom advanced prompt (null to use default)
    pub advanced_custom: Option<String>,
    /// Whether dictionary section is enabled
    pub dictionary_enabled: bool,
    /// Custom dictionary prompt (null to use default)
    pub dictionary_custom: Option<String>,
}

impl From<PromptConfigPayload> for PromptSections {
    fn from(payload: PromptConfigPayload) -> Self {
        Self {
            main_custom: payload.main_custom,
            advanced_enabled: payload.advanced_enabled,
            advanced_custom: payload.advanced_custom,
            dictionary_enabled: payload.dictionary_enabled,
            dictionary_custom: payload.dictionary_custom,
        }
    }
}

impl From<PromptSections> for PromptConfigPayload {
    fn from(sections: PromptSections) -> Self {
        Self {
            main_custom: sections.main_custom,
            advanced_enabled: sections.advanced_enabled,
            advanced_custom: sections.advanced_custom,
            dictionary_enabled: sections.dictionary_enabled,
            dictionary_custom: sections.dictionary_custom,
        }
    }
}

/// Get the default prompt templates
#[tauri::command]
pub fn get_llm_default_prompts() -> DefaultPromptsResponse {
    DefaultPromptsResponse {
        main: MAIN_PROMPT_DEFAULT.to_string(),
        advanced: ADVANCED_PROMPT_DEFAULT.to_string(),
        dictionary: DICTIONARY_PROMPT_DEFAULT.to_string(),
    }
}

/// Response containing default prompts
#[derive(Debug, serde::Serialize)]
pub struct DefaultPromptsResponse {
    pub main: String,
    pub advanced: String,
    pub dictionary: String,
}

/// Get available LLM providers
#[tauri::command]
pub fn get_llm_providers() -> Vec<LlmProviderInfo> {
    vec![
        LlmProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            requires_api_key: true,
            default_model: "gpt-4o-mini".to_string(),
            models: vec![
                "gpt-4o-mini".to_string(),
                "gpt-4o".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            requires_api_key: true,
            default_model: "claude-3-haiku-20240307".to_string(),
            models: vec![
                "claude-3-haiku-20240307".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-opus-20240229".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama".to_string(),
            requires_api_key: false,
            default_model: "llama3.2".to_string(),
            models: vec![
                "llama3.2".to_string(),
                "llama3.1".to_string(),
                "mistral".to_string(),
                "codellama".to_string(),
            ],
        },
    ]
}

/// LLM provider information for the frontend
#[derive(Debug, serde::Serialize)]
pub struct LlmProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_api_key: bool,
    pub default_model: String,
    pub models: Vec<String>,
}

/// Update LLM configuration on the pipeline
#[tauri::command]
pub fn update_llm_config(
    pipeline: State<'_, SharedPipeline>,
    config: LlmConfigPayload,
) -> Result<(), LlmCommandError> {
    // Get current pipeline config and update just the LLM portion
    // Note: This is a simplified approach - in a full implementation,
    // we'd want to preserve other config and only update LLM settings
    let llm_config = LlmConfig {
        enabled: config.enabled,
        provider: config.provider,
        api_key: config.api_key.unwrap_or_default(),
        model: config.model,
        ollama_url: config.ollama_url,
        prompts: PromptSections::default(),
        program_prompt_profiles: Vec::new(),
        timeout: Duration::from_secs(config.timeout_secs.unwrap_or(30)),
    };

    // Get current config from pipeline and update LLM portion
    let current_config = get_current_pipeline_config(&pipeline)?;
    let new_config = crate::pipeline::PipelineConfig {
        llm_config,
        ..current_config
    };

    pipeline
        .update_config(new_config)
        .map_err(|e| LlmCommandError::from(e.to_string()))?;

    log::info!("LLM configuration updated");
    Ok(())
}

/// Update LLM prompt configuration
#[tauri::command]
pub fn update_llm_prompts(
    pipeline: State<'_, SharedPipeline>,
    prompts: PromptConfigPayload,
) -> Result<(), LlmCommandError> {
    let current_config = get_current_pipeline_config(&pipeline)?;
    let mut llm_config = current_config.llm_config.clone();
    llm_config.prompts = prompts.into();

    let new_config = crate::pipeline::PipelineConfig {
        llm_config,
        ..current_config
    };

    pipeline
        .update_config(new_config)
        .map_err(|e| LlmCommandError::from(e.to_string()))?;

    log::info!("LLM prompts updated");
    Ok(())
}

/// Get current LLM configuration
#[tauri::command]
pub fn get_llm_config(pipeline: State<'_, SharedPipeline>) -> Result<LlmConfigResponse, LlmCommandError> {
    let config = get_current_pipeline_config(&pipeline)?;
    Ok(LlmConfigResponse {
        enabled: config.llm_config.enabled,
        provider: config.llm_config.provider,
        model: config.llm_config.model,
        ollama_url: config.llm_config.ollama_url,
        timeout_secs: config.llm_config.timeout.as_secs(),
        prompts: config.llm_config.prompts.into(),
    })
}

/// Response containing current LLM configuration
#[derive(Debug, serde::Serialize)]
pub struct LlmConfigResponse {
    pub enabled: bool,
    pub provider: String,
    pub model: Option<String>,
    pub ollama_url: Option<String>,
    pub timeout_secs: u64,
    pub prompts: PromptConfigPayload,
}

/// Helper to get current pipeline config (placeholder - needs proper implementation)
fn get_current_pipeline_config(
    _pipeline: &State<'_, SharedPipeline>,
) -> Result<crate::pipeline::PipelineConfig, LlmCommandError> {
    // Note: The current SharedPipeline doesn't expose config reading
    // For now, return default config. In a full implementation, we'd
    // add a get_config() method to SharedPipeline.
    Ok(crate::pipeline::PipelineConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_llm_providers() {
        let providers = get_llm_providers();
        assert_eq!(providers.len(), 3);
        assert!(providers.iter().any(|p| p.id == "openai"));
        assert!(providers.iter().any(|p| p.id == "anthropic"));
        assert!(providers.iter().any(|p| p.id == "ollama"));
    }

    #[test]
    fn test_get_default_prompts() {
        let prompts = get_llm_default_prompts();
        assert!(!prompts.main.is_empty());
        assert!(!prompts.advanced.is_empty());
        assert!(!prompts.dictionary.is_empty());
    }
}
