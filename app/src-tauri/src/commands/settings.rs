use crate::settings::HotkeyConfig;
use tauri::{AppHandle, Manager};

#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg(desktop)]
use tauri_plugin_store::StoreExt;

/// Temporarily unregister all global shortcuts.
/// Call this before capturing a new hotkey to prevent the shortcuts from intercepting key presses.
#[cfg(desktop)]
#[tauri::command]
pub async fn unregister_shortcuts(app: AppHandle) -> Result<(), String> {
    log::info!("Temporarily unregistering all shortcuts for hotkey capture");
    let shortcut_manager = app.global_shortcut();
    shortcut_manager
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;
    Ok(())
}

// Stub for non-desktop platforms
#[cfg(not(desktop))]
#[tauri::command]
pub async fn unregister_shortcuts(_app: AppHandle) -> Result<(), String> {
    Ok(())
}

/// Helper to read a setting from the store with a default fallback
#[cfg(desktop)]
fn get_setting_from_store<T: serde::de::DeserializeOwned>(
    app: &AppHandle,
    key: &str,
    default: T,
) -> T {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get(key))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(default)
}

/// Re-register global shortcuts with the current settings from the store.
/// Called from frontend after hotkey settings are changed.
/// Falls back to defaults if stored values are invalid.
#[cfg(desktop)]
#[tauri::command]
pub async fn register_shortcuts(app: AppHandle) -> Result<(), String> {
    // Read hotkeys from store with defaults
    let toggle_hotkey: HotkeyConfig =
        get_setting_from_store(&app, "toggle_hotkey", HotkeyConfig::default_toggle());
    let hold_hotkey: HotkeyConfig =
        get_setting_from_store(&app, "hold_hotkey", HotkeyConfig::default_hold());
    let paste_last_hotkey: HotkeyConfig = get_setting_from_store(
        &app,
        "paste_last_hotkey",
        HotkeyConfig::default_paste_last(),
    );

    // Convert to shortcuts with validation (fall back to defaults if invalid)
    let toggle_shortcut = toggle_hotkey.to_shortcut_or_default(HotkeyConfig::default_toggle);
    let hold_shortcut = hold_hotkey.to_shortcut_or_default(HotkeyConfig::default_hold);
    let paste_last_shortcut =
        paste_last_hotkey.to_shortcut_or_default(HotkeyConfig::default_paste_last);

    log::info!(
        "Re-registering shortcuts - Toggle: {}, Hold: {}, PasteLast: {}",
        toggle_hotkey.to_shortcut_string(),
        hold_hotkey.to_shortcut_string(),
        paste_last_hotkey.to_shortcut_string()
    );

    // Get the global shortcut manager
    let shortcut_manager = app.global_shortcut();

    // Unregister all existing shortcuts
    shortcut_manager
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    // Collect shortcuts to register
    let shortcuts: Vec<Shortcut> = vec![toggle_shortcut, hold_shortcut, paste_last_shortcut];

    // Register new shortcuts with handler
    shortcut_manager
        .on_shortcuts(shortcuts, |app, shortcut, event| {
            crate::handle_shortcut_event(app, shortcut, &event);
        })
        .map_err(|e| format!("Failed to register shortcuts: {}", e))?;

    // If we're currently recording/transcribing, re-enable Escape-to-cancel.
    // (Registering hotkeys unregisters all shortcuts, which would otherwise drop Escape.)
    let should_enable_escape = app
        .try_state::<crate::pipeline::SharedPipeline>()
        .map(|p| p.state().can_cancel())
        .unwrap_or(false);
    crate::set_escape_cancel_shortcut_enabled(&app, should_enable_escape);

    log::info!("Shortcuts re-registered successfully");
    Ok(())
}

// Stub for non-desktop platforms
#[cfg(not(desktop))]
#[tauri::command]
pub async fn register_shortcuts(_app: AppHandle) -> Result<(), String> {
    Ok(())
}
