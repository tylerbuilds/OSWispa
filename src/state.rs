//! Truthful, UI-agnostic application lifecycle state.
//!
//! Platform surfaces consume [`AppPhase`] snapshots. The reducer deliberately
//! contains no I/O so lifecycle behaviour can be proved without a microphone,
//! clipboard, window server, or desktop notification service.

/// The result of delivering a completed transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// The transcript was copied and inserted into the focused application.
    Inserted,
    /// The transcript is on the clipboard but was not inserted.
    CopiedOnly,
    /// The transcript could not be copied or inserted.
    Failed,
}

/// The user-visible lifecycle of one dictation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppPhase {
    #[default]
    Booting,
    Ready,
    /// A capture request was accepted, but the audio backend is not live yet.
    Arming,
    /// The audio backend confirmed that capture is live.
    Listening {
        device_name: String,
    },
    /// Capture has stopped and audio is being prepared or transcribed.
    Processing,
    /// A transcript exists and OSWispa is attempting clipboard/text delivery.
    Delivering,
    Delivered(DeliveryOutcome),
    Cancelled,
    NeedsAttention,
}

impl AppPhase {
    /// Whether a stop or cancel command can apply to the active capture.
    pub fn is_capturing(&self) -> bool {
        matches!(self, Self::Arming | Self::Listening { .. })
    }
}

/// Lifecycle facts emitted by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    WorkersReady,
    StartRequested,
    CaptureStarted { device_name: String },
    StopRequested,
    CancelRequested,
    TranscriptionReady,
    DeliveryFinished(DeliveryOutcome),
    Failed,
}

/// Reduce a lifecycle fact into the next truthful phase.
///
/// Invalid or stale facts leave the phase unchanged. In particular, a late
/// capture acknowledgement cannot move an already-processing attempt back to
/// `Listening`.
pub fn reduce_phase(current: &AppPhase, event: LifecycleEvent) -> AppPhase {
    match (current, event) {
        (AppPhase::Booting, LifecycleEvent::WorkersReady) => AppPhase::Ready,

        (
            AppPhase::Ready
            | AppPhase::Delivered(_)
            | AppPhase::Cancelled
            | AppPhase::NeedsAttention,
            LifecycleEvent::StartRequested,
        ) => AppPhase::Arming,

        (AppPhase::Arming, LifecycleEvent::CaptureStarted { device_name }) => {
            AppPhase::Listening { device_name }
        }

        (AppPhase::Arming | AppPhase::Listening { .. }, LifecycleEvent::StopRequested) => {
            AppPhase::Processing
        }

        (AppPhase::Arming | AppPhase::Listening { .. }, LifecycleEvent::CancelRequested) => {
            AppPhase::Cancelled
        }

        (AppPhase::Processing, LifecycleEvent::TranscriptionReady) => AppPhase::Delivering,

        (AppPhase::Delivering, LifecycleEvent::DeliveryFinished(outcome)) => {
            AppPhase::Delivered(outcome)
        }

        (_, LifecycleEvent::Failed) => AppPhase::NeedsAttention,
        _ => current.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_capture_is_not_reported_as_listening() {
        let phase = reduce_phase(&AppPhase::Ready, LifecycleEvent::StartRequested);
        assert_eq!(phase, AppPhase::Arming);
    }

    #[test]
    fn capture_acknowledgement_names_the_live_device() {
        let phase = reduce_phase(
            &AppPhase::Arming,
            LifecycleEvent::CaptureStarted {
                device_name: "Studio microphone".to_string(),
            },
        );
        assert_eq!(
            phase,
            AppPhase::Listening {
                device_name: "Studio microphone".to_string()
            }
        );
    }

    #[test]
    fn late_capture_acknowledgement_cannot_regress_processing() {
        let phase = reduce_phase(
            &AppPhase::Processing,
            LifecycleEvent::CaptureStarted {
                device_name: "Late device".to_string(),
            },
        );
        assert_eq!(phase, AppPhase::Processing);
    }

    #[test]
    fn stop_transcription_and_delivery_have_distinct_phases() {
        let processing = reduce_phase(
            &AppPhase::Listening {
                device_name: "Default".to_string(),
            },
            LifecycleEvent::StopRequested,
        );
        assert_eq!(processing, AppPhase::Processing);

        let delivering = reduce_phase(&processing, LifecycleEvent::TranscriptionReady);
        assert_eq!(delivering, AppPhase::Delivering);

        let delivered = reduce_phase(
            &delivering,
            LifecycleEvent::DeliveryFinished(DeliveryOutcome::Inserted),
        );
        assert_eq!(delivered, AppPhase::Delivered(DeliveryOutcome::Inserted));
    }

    #[test]
    fn delivery_outcomes_remain_distinguishable() {
        for outcome in [
            DeliveryOutcome::Inserted,
            DeliveryOutcome::CopiedOnly,
            DeliveryOutcome::Failed,
        ] {
            assert_eq!(
                reduce_phase(
                    &AppPhase::Delivering,
                    LifecycleEvent::DeliveryFinished(outcome),
                ),
                AppPhase::Delivered(outcome)
            );
        }
    }

    #[test]
    fn cancellation_and_failure_are_terminal_but_retryable() {
        let cancelled = reduce_phase(&AppPhase::Arming, LifecycleEvent::CancelRequested);
        assert_eq!(cancelled, AppPhase::Cancelled);
        assert_eq!(
            reduce_phase(&cancelled, LifecycleEvent::StartRequested),
            AppPhase::Arming
        );

        let failed = reduce_phase(&AppPhase::Processing, LifecycleEvent::Failed);
        assert_eq!(failed, AppPhase::NeedsAttention);
        assert_eq!(
            reduce_phase(&failed, LifecycleEvent::StartRequested),
            AppPhase::Arming
        );
    }
}
