//! Request logging system for debugging and troubleshooting.
//!
//! Captures detailed logs for each transcription request including:
//! - Request metadata (timestamp, provider, model)
//! - Audio information (duration, sample rate, size)
//! - API request/response details
//! - Timing information
//! - Errors if any

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Maximum number of request logs to keep in memory
const MAX_LOGS: usize = 100;

/// A single log entry within a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub details: Option<String>,
}

/// Log level for entries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A complete request log containing all entries for a single transcription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    /// Unique ID for this request
    pub id: String,
    /// When the request started
    pub started_at: DateTime<Utc>,
    /// When the request completed (if finished)
    #[serde(rename = "ended_at")]
    pub completed_at: Option<DateTime<Utc>>,
    /// STT provider used
    pub stt_provider: String,
    /// STT model used
    pub stt_model: Option<String>,
    /// LLM provider used (if formatting enabled)
    pub llm_provider: Option<String>,
    /// LLM model used
    pub llm_model: Option<String>,
    /// Audio duration in seconds
    pub audio_duration_secs: Option<f32>,
    /// Audio file size in bytes
    pub audio_size_bytes: Option<usize>,
    /// Sample rate of the audio
    pub sample_rate: Option<u32>,
    /// Raw transcript from STT
    pub raw_transcript: Option<String>,
    /// Formatted transcript from LLM (if used)
    #[serde(rename = "final_text")]
    pub formatted_transcript: Option<String>,
    /// Final result (success or error)
    pub status: RequestStatus,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// All log entries for this request
    pub entries: Vec<LogEntry>,
    /// Total duration in milliseconds
    pub total_duration_ms: Option<u64>,
    /// STT duration in milliseconds
    pub stt_duration_ms: Option<u64>,
    /// LLM duration in milliseconds
    pub llm_duration_ms: Option<u64>,
}

/// Status of a request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    /// Request is in progress
    InProgress,
    /// Request completed successfully
    Success,
    /// Request failed
    Error,
    /// Request was cancelled
    Cancelled,
}

impl RequestLog {
    /// Create a new request log
    pub fn new(stt_provider: String, stt_model: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            completed_at: None,
            stt_provider,
            stt_model,
            llm_provider: None,
            llm_model: None,
            audio_duration_secs: None,
            audio_size_bytes: None,
            sample_rate: None,
            raw_transcript: None,
            formatted_transcript: None,
            status: RequestStatus::InProgress,
            error_message: None,
            entries: Vec::new(),
            total_duration_ms: None,
            stt_duration_ms: None,
            llm_duration_ms: None,
        }
    }

    /// Add a log entry
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>, details: Option<String>) {
        self.entries.push(LogEntry {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            details,
        });
    }

    /// Log debug message
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn debug(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Debug, message, None);
    }

    /// Log info message
    pub fn info(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Info, message, None);
    }

    /// Log warning message
    pub fn warn(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message, None);
    }

    /// Log error message
    pub fn error(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Error, message, None);
    }

    /// Log with details
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn info_with_details(&mut self, message: impl Into<String>, details: impl Into<String>) {
        self.log(LogLevel::Info, message, Some(details.into()));
    }

    /// Mark request as complete with success
    pub fn complete_success(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = RequestStatus::Success;
        self.total_duration_ms = Some(
            (self.completed_at.unwrap() - self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Mark request as complete with error
    pub fn complete_error(&mut self, error: impl Into<String>) {
        self.completed_at = Some(Utc::now());
        self.status = RequestStatus::Error;
        self.error_message = Some(error.into());
        self.total_duration_ms = Some(
            (self.completed_at.unwrap() - self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Mark request as cancelled
    pub fn complete_cancelled(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = RequestStatus::Cancelled;
        self.total_duration_ms = Some(
            (self.completed_at.unwrap() - self.started_at)
                .num_milliseconds() as u64,
        );
    }
}

/// Thread-safe request log store
#[derive(Debug, Clone)]
pub struct RequestLogStore {
    logs: Arc<Mutex<VecDeque<RequestLog>>>,
    current: Arc<Mutex<Option<RequestLog>>>,
}

impl Default for RequestLogStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestLogStore {
    /// Create a new log store
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOGS))),
            current: Arc::new(Mutex::new(None)),
        }
    }

    /// Start a new request log
    pub fn start_request(&self, stt_provider: String, stt_model: Option<String>) -> String {
        let mut current = self.current.lock().unwrap();

        // If there's an existing request, finalize it first
        if let Some(mut existing) = current.take() {
            if existing.status == RequestStatus::InProgress {
                existing.complete_cancelled();
            }
            self.store_log(existing);
        }

        let log = RequestLog::new(stt_provider, stt_model);
        let id = log.id.clone();
        *current = Some(log);
        id
    }

    /// Get the current request log for modification
    pub fn with_current<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut RequestLog) -> R,
    {
        let mut current = self.current.lock().unwrap();
        current.as_mut().map(f)
    }

    /// Complete the current request and store it
    pub fn complete_current(&self) {
        let mut current = self.current.lock().unwrap();
        if let Some(log) = current.take() {
            self.store_log(log);
        }
    }

    /// Store a completed log
    fn store_log(&self, log: RequestLog) {
        let mut logs = self.logs.lock().unwrap();
        if logs.len() >= MAX_LOGS {
            logs.pop_front();
        }
        logs.push_back(log);
    }

    /// Get all stored logs (most recent first)
    pub fn get_logs(&self, limit: Option<usize>) -> Vec<RequestLog> {
        let logs = self.logs.lock().unwrap();
        let current = self.current.lock().unwrap();

        let mut result: Vec<RequestLog> = logs.iter().cloned().collect();

        // Add current request if exists
        if let Some(ref c) = *current {
            result.push(c.clone());
        }

        // Reverse to get most recent first
        result.reverse();

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        result
    }

    /// Clear all logs
    pub fn clear(&self) {
        let mut logs = self.logs.lock().unwrap();
        logs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_log_creation() {
        let log = RequestLog::new("groq".to_string(), Some("whisper-large-v3".to_string()));
        assert_eq!(log.stt_provider, "groq");
        assert_eq!(log.stt_model, Some("whisper-large-v3".to_string()));
        assert_eq!(log.status, RequestStatus::InProgress);
        assert_eq!(log.error_message, None);
    }

    #[test]
    fn test_log_entries() {
        let mut log = RequestLog::new("groq".to_string(), None);
        log.info("Recording started");
        log.debug("Audio buffer initialized");
        log.error("API call failed");

        assert_eq!(log.entries.len(), 3);
        assert_eq!(log.entries[0].level, LogLevel::Info);
        assert_eq!(log.entries[1].level, LogLevel::Debug);
        assert_eq!(log.entries[2].level, LogLevel::Error);
    }

    #[test]
    fn test_log_store() {
        let store = RequestLogStore::new();

        let id1 = store.start_request("groq".to_string(), None);
        store.with_current(|log| {
            log.info("Test message");
            log.complete_success();
        });
        store.complete_current();

        let id2 = store.start_request("openai".to_string(), None);
        store.with_current(|log| {
            log.info("Another test");
            log.complete_success();
        });
        store.complete_current();

        let logs = store.get_logs(None);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].id, id2); // Most recent first
        assert_eq!(logs[1].id, id1);
    }
}
