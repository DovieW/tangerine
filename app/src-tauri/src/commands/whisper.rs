//! Tauri commands for local Whisper model management.
//!
//! These commands are only available when the `local-whisper` feature is enabled.

#[cfg(feature = "local-whisper")]
use crate::stt::{LocalWhisperConfig, LocalWhisperProvider, WhisperModel};
use std::path::PathBuf;
use tauri::Manager;

/// Error type for Whisper commands
#[derive(Debug, serde::Serialize)]
pub struct WhisperCommandError {
    pub message: String,
}

impl From<String> for WhisperCommandError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

/// Information about a Whisper model
#[derive(Debug, serde::Serialize)]
pub struct WhisperModelInfo {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub download_url: String,
    pub is_english_only: bool,
    pub is_downloaded: bool,
}

/// Check if local Whisper feature is enabled
#[tauri::command]
pub fn is_local_whisper_available() -> bool {
    cfg!(feature = "local-whisper")
}

/// Get list of available Whisper models with download status
#[tauri::command]
pub fn get_whisper_models(app: tauri::AppHandle) -> Result<Vec<WhisperModelInfo>, WhisperCommandError> {
    #[cfg(feature = "local-whisper")]
    {
        let models_dir = get_models_dir(&app)?;

        let models: Vec<WhisperModelInfo> = WhisperModel::all()
            .into_iter()
            .map(|model| {
                let model_path = models_dir.join(model.filename());
                let is_downloaded = model_path.exists();

                WhisperModelInfo {
                    id: format!("{:?}", model).to_lowercase(),
                    name: model.display_name().to_string(),
                    filename: model.filename().to_string(),
                    size_bytes: model.size_bytes(),
                    size_display: format_size(model.size_bytes()),
                    download_url: model.download_url(),
                    is_english_only: model.is_english_only(),
                    is_downloaded,
                }
            })
            .collect();

        Ok(models)
    }

    #[cfg(not(feature = "local-whisper"))]
    {
        let _ = app;
        Err(WhisperCommandError::from(
            "Local Whisper feature is not enabled".to_string(),
        ))
    }
}

/// Get the path to the models directory
#[tauri::command]
pub fn get_whisper_models_dir(app: tauri::AppHandle) -> Result<String, WhisperCommandError> {
    let models_dir = get_models_dir(&app)?;
    Ok(models_dir.to_string_lossy().to_string())
}

/// Check if a specific model is downloaded
#[tauri::command]
pub fn is_whisper_model_downloaded(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<bool, WhisperCommandError> {
    #[cfg(feature = "local-whisper")]
    {
        let model = parse_model_id(&model_id)?;
        let models_dir = get_models_dir(&app)?;
        let model_path = models_dir.join(model.filename());
        Ok(model_path.exists())
    }

    #[cfg(not(feature = "local-whisper"))]
    {
        let _ = (app, model_id);
        Err(WhisperCommandError::from(
            "Local Whisper feature is not enabled".to_string(),
        ))
    }
}

/// Get the download URL for a model
#[tauri::command]
pub fn get_whisper_model_url(model_id: String) -> Result<String, WhisperCommandError> {
    #[cfg(feature = "local-whisper")]
    {
        let model = parse_model_id(&model_id)?;
        Ok(model.download_url())
    }

    #[cfg(not(feature = "local-whisper"))]
    {
        let _ = model_id;
        Err(WhisperCommandError::from(
            "Local Whisper feature is not enabled".to_string(),
        ))
    }
}

/// Delete a downloaded model
#[tauri::command]
pub fn delete_whisper_model(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<(), WhisperCommandError> {
    #[cfg(feature = "local-whisper")]
    {
        let model = parse_model_id(&model_id)?;
        let models_dir = get_models_dir(&app)?;
        let model_path = models_dir.join(model.filename());

        if model_path.exists() {
            std::fs::remove_file(&model_path).map_err(|e| {
                WhisperCommandError::from(format!("Failed to delete model: {}", e))
            })?;
            log::info!("Deleted Whisper model: {}", model_path.display());
        }

        Ok(())
    }

    #[cfg(not(feature = "local-whisper"))]
    {
        let _ = (app, model_id);
        Err(WhisperCommandError::from(
            "Local Whisper feature is not enabled".to_string(),
        ))
    }
}

/// Validate a downloaded model file
#[tauri::command]
pub fn validate_whisper_model(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<bool, WhisperCommandError> {
    #[cfg(feature = "local-whisper")]
    {
        let model = parse_model_id(&model_id)?;
        let models_dir = get_models_dir(&app)?;
        let model_path = models_dir.join(model.filename());

        if !model_path.exists() {
            return Ok(false);
        }

        // Check file size is reasonable (at least 50% of expected)
        let metadata = std::fs::metadata(&model_path).map_err(|e| {
            WhisperCommandError::from(format!("Failed to read model metadata: {}", e))
        })?;

        let expected_size = model.size_bytes();
        let actual_size = metadata.len();

        // Model should be at least 50% of expected size
        if actual_size < expected_size / 2 {
            log::warn!(
                "Model {} appears incomplete: {} bytes (expected ~{} bytes)",
                model_id,
                actual_size,
                expected_size
            );
            return Ok(false);
        }

        Ok(true)
    }

    #[cfg(not(feature = "local-whisper"))]
    {
        let _ = (app, model_id);
        Err(WhisperCommandError::from(
            "Local Whisper feature is not enabled".to_string(),
        ))
    }
}

// Helper functions

fn get_models_dir(app: &tauri::AppHandle) -> Result<PathBuf, WhisperCommandError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| WhisperCommandError::from(format!("Failed to get app data dir: {}", e)))?;

    let models_dir = app_data_dir.join("whisper-models");

    // Create directory if it doesn't exist
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|e| {
            WhisperCommandError::from(format!("Failed to create models directory: {}", e))
        })?;
    }

    Ok(models_dir)
}

#[cfg(feature = "local-whisper")]
fn parse_model_id(model_id: &str) -> Result<WhisperModel, WhisperCommandError> {
    let model = match model_id.to_lowercase().as_str() {
        "tiny" => WhisperModel::Tiny,
        "tinyen" | "tiny_en" | "tiny-en" => WhisperModel::TinyEn,
        "base" => WhisperModel::Base,
        "baseen" | "base_en" | "base-en" => WhisperModel::BaseEn,
        "small" => WhisperModel::Small,
        "smallen" | "small_en" | "small-en" => WhisperModel::SmallEn,
        "medium" => WhisperModel::Medium,
        "mediumen" | "medium_en" | "medium-en" => WhisperModel::MediumEn,
        "largev1" | "large_v1" | "large-v1" => WhisperModel::LargeV1,
        "largev2" | "large_v2" | "large-v2" => WhisperModel::LargeV2,
        "largev3" | "large_v3" | "large-v3" => WhisperModel::LargeV3,
        "largev3turbo" | "large_v3_turbo" | "large-v3-turbo" => WhisperModel::LargeV3Turbo,
        _ => {
            return Err(WhisperCommandError::from(format!(
                "Unknown model: {}",
                model_id
            )));
        }
    };
    Ok(model)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(75_000_000), "72 MB"); // 75M / 1024 / 1024 ≈ 71.5 → rounds to 72
        assert_eq!(format_size(1_500_000_000), "1.4 GB");
    }

    #[test]
    fn test_is_local_whisper_available() {
        // This will be true or false depending on feature flag
        let _ = is_local_whisper_available();
    }
}
