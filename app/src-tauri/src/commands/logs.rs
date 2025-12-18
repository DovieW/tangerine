//! Tauri commands for request logging.

use crate::request_log::{RequestLog, RequestLogStore};
use tauri::{AppHandle, Manager};

/// Get all request logs
#[tauri::command]
pub fn get_request_logs(app: AppHandle, limit: Option<usize>) -> Vec<RequestLog> {
    if let Some(store) = app.try_state::<RequestLogStore>() {
        store.get_logs(limit)
    } else {
        Vec::new()
    }
}

/// Clear all request logs
#[tauri::command]
pub fn clear_request_logs(app: AppHandle) {
    if let Some(store) = app.try_state::<RequestLogStore>() {
        store.clear();
    }
}
