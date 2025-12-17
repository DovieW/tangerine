//! Edge case tests for the recording pipeline.
//!
//! These tests verify behavior under stress conditions:
//! - State machine guards
//! - Configuration defaults
//! - Audio format handling
//! - VAD behavior
//!
//! Note: Tests that require actual audio capture hardware are marked #[ignore]
//! and should be run manually with `cargo test -- --ignored`

use crate::pipeline::{PipelineConfig, PipelineError, PipelineState, SharedPipeline};
use crate::stt::RetryConfig;
use std::sync::Arc;
use std::time::Duration;

/// Test PipelineState guards.
#[test]
fn test_pipeline_state_guards() {
    // Test Idle state
    let idle = PipelineState::Idle;
    assert!(idle.can_start_recording());
    assert!(!idle.can_stop_recording());
    assert!(!idle.can_cancel());

    // Test Recording state
    let recording = PipelineState::Recording;
    assert!(!recording.can_start_recording());
    assert!(recording.can_stop_recording());
    assert!(recording.can_cancel());

    // Test Transcribing state
    let transcribing = PipelineState::Transcribing;
    assert!(!transcribing.can_start_recording());
    assert!(!transcribing.can_stop_recording());
    assert!(transcribing.can_cancel());

    // Test Error state
    let error = PipelineState::Error;
    assert!(error.can_start_recording());
    assert!(!error.can_stop_recording());
    assert!(!error.can_cancel());
}

/// Test PipelineConfig default values.
#[test]
fn test_pipeline_config_defaults() {
    let config = PipelineConfig::default();

    // Verify reasonable defaults
    assert!(config.max_duration_secs > 0.0);
    assert!(config.max_recording_bytes > 0);
    assert!(config.transcription_timeout.as_secs() > 0);
    assert!(config.retry_config.max_retries > 0);
}

/// Test PipelineConfig custom values.
#[test]
fn test_pipeline_config_custom() {
    let config = PipelineConfig {
        max_duration_secs: 120.0,
        max_recording_bytes: 100_000_000,
        transcription_timeout: Duration::from_secs(60),
        retry_config: RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            retry_on_rate_limit: true,
        },
        ..Default::default()
    };

    assert_eq!(config.max_duration_secs, 120.0);
    assert_eq!(config.max_recording_bytes, 100_000_000);
    assert_eq!(config.transcription_timeout, Duration::from_secs(60));
    assert_eq!(config.retry_config.max_retries, 5);
}

/// Test RetryConfig defaults.
#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();

    assert!(config.max_retries > 0);
    assert!(config.initial_delay.as_millis() > 0);
    assert!(config.max_delay > config.initial_delay);
}

/// Test pipeline creation doesn't panic.
#[test]
fn test_pipeline_creation() {
    let config = PipelineConfig::default();
    let _pipeline = SharedPipeline::new(config);
    // Just verify it doesn't panic
}

/// Test pipeline initial state is Idle.
#[test]
fn test_pipeline_initial_state() {
    let config = PipelineConfig::default();
    let pipeline = SharedPipeline::new(config);

    assert_eq!(pipeline.state(), PipelineState::Idle);
    assert!(!pipeline.is_recording());
    assert!(!pipeline.is_error());
}

/// Test that stopping when not recording returns NotRecording error.
#[test]
fn test_stop_when_not_recording() {
    let config = PipelineConfig::default();
    let pipeline = SharedPipeline::new(config);

    // Stop without starting should fail
    let stop_result = pipeline.stop_recording();
    assert!(
        matches!(stop_result, Err(PipelineError::NotRecording)),
        "Stop without start should return NotRecording error, got {:?}",
        stop_result
    );
}

/// Test concurrent state queries don't deadlock.
#[tokio::test]
async fn test_concurrent_state_queries() {
    let config = PipelineConfig::default();
    let pipeline = Arc::new(SharedPipeline::new(config));

    // Spawn multiple tasks that query state
    let mut handles = vec![];

    for _ in 0..10 {
        let p = Arc::clone(&pipeline);
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = p.state();
                let _ = p.is_recording();
                let _ = p.is_error();
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks - should not deadlock
    for handle in handles {
        handle.await.expect("Task should complete without panic");
    }
}

/// Test force_reset resets pipeline state.
#[test]
fn test_force_reset() {
    let config = PipelineConfig::default();
    let pipeline = SharedPipeline::new(config);

    // Force reset should work even on idle pipeline
    pipeline.force_reset();
    assert_eq!(pipeline.state(), PipelineState::Idle);
}

// ============================================================
// Tests that require audio hardware - marked as ignored
// Run with: cargo test -- --ignored
// ============================================================

/// Test rapid start/stop cycles (requires audio hardware).
#[test]
#[ignore]
fn test_rapid_start_stop_cycles() {
    let config = PipelineConfig::default();
    let pipeline = SharedPipeline::new(config);

    for _ in 0..10 {
        let _ = pipeline.start_recording();
        pipeline.cancel();
    }

    let state = pipeline.state();
    assert!(
        matches!(state, PipelineState::Idle | PipelineState::Error),
        "Pipeline should be Idle or Error after rapid cycles"
    );
}

/// Test double start error (requires audio hardware).
#[test]
#[ignore]
fn test_double_start_error() {
    let config = PipelineConfig::default();
    let pipeline = SharedPipeline::new(config);

    let first = pipeline.start_recording();
    assert!(first.is_ok(), "First start should succeed");

    let second = pipeline.start_recording();
    assert!(
        matches!(second, Err(PipelineError::AlreadyRecording)),
        "Second start should fail"
    );

    pipeline.cancel();
}

#[cfg(test)]
mod audio_format_tests {
    use crate::stt::{AudioEncoding, AudioFormat};

    #[test]
    fn test_audio_format_creation() {
        let format = AudioFormat {
            sample_rate: 16000,
            channels: 1,
            encoding: AudioEncoding::Wav,
        };

        assert_eq!(format.sample_rate, 16000);
        assert_eq!(format.channels, 1);
    }

    #[test]
    fn test_audio_format_various_rates() {
        // Test common sample rates
        for rate in [8000, 16000, 22050, 44100, 48000] {
            let format = AudioFormat {
                sample_rate: rate,
                channels: 1,
                encoding: AudioEncoding::Wav,
            };
            assert_eq!(format.sample_rate, rate);
        }
    }

    #[test]
    fn test_audio_encoding_variants() {
        // Ensure all variants can be used
        let _wav = AudioEncoding::Wav;
        let _pcm = AudioEncoding::Pcm16;
    }
}

#[cfg(test)]
mod vad_edge_case_tests {
    use crate::vad::{VadConfig, VadEvent, VoiceActivityDetector};

    #[test]
    fn test_vad_with_silence() {
        let config = VadConfig::default();
        let mut vad = VoiceActivityDetector::new(config);

        // Process silence - should not detect speech
        // Frame size for 10ms at 16kHz = 160 samples
        let silence = vec![0i16; 160];
        let event = vad.process_frame(&silence);

        // Silence should not trigger speech start
        assert!(
            !matches!(event, VadEvent::SpeechStart { .. }),
            "Silence should not trigger speech start"
        );
    }

    #[test]
    fn test_vad_config_defaults() {
        let config = VadConfig::default();

        // Verify reasonable defaults
        assert!(config.speech_frames_threshold > 0);
        assert!(config.hangover_frames > 0);
        assert!(config.pre_roll_ms > 0);
        assert!(config.frame_duration_ms > 0);
        assert!(config.sample_rate > 0);
    }

    #[test]
    fn test_vad_multiple_silence_frames() {
        let config = VadConfig::default();
        let mut vad = VoiceActivityDetector::new(config);

        // Process multiple frames of silence
        let silence = vec![0i16; 160];
        for _ in 0..10 {
            let event = vad.process_frame(&silence);
            // Should never start speech on pure silence
            assert!(
                !matches!(event, VadEvent::SpeechStart { .. }),
                "Should not detect speech in silence"
            );
        }
    }

    #[test]
    fn test_vad_creation_with_custom_config() {
        let config = VadConfig {
            speech_frames_threshold: 5,
            hangover_frames: 50,
            pre_roll_ms: 500,
            frame_duration_ms: 20,
            sample_rate: 16000,
            ..Default::default()
        };

        let vad = VoiceActivityDetector::new(config);
        // Just verify creation doesn't panic
        assert!(!vad.is_speaking());
    }
}
