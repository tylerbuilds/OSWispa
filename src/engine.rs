//! Public command and lifecycle boundary for desktop shells.

use crate::state::{AppPhase, DeliveryOutcome};
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender, TrySendError};
use serde::{Deserialize, Serialize};
use std::thread::JoinHandle;

/// Commands accepted by a running OSWispa engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineCommand {
    Start,
    Stop,
    Cancel,
    Reload,
    Shutdown,
}

/// Transcript-free lifecycle phases safe to expose in routine desktop state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnginePhase {
    Booting,
    Ready,
    Arming,
    Listening,
    Processing,
    Delivering,
    Delivered(DeliveryOutcome),
    Cancelled,
    NeedsAttention,
    Stopped,
}

impl From<&AppPhase> for EnginePhase {
    fn from(phase: &AppPhase) -> Self {
        match phase {
            AppPhase::Booting => Self::Booting,
            AppPhase::Ready => Self::Ready,
            AppPhase::Arming => Self::Arming,
            AppPhase::Listening { .. } => Self::Listening,
            AppPhase::Processing => Self::Processing,
            AppPhase::Delivering => Self::Delivering,
            AppPhase::Delivered(outcome) => Self::Delivered(*outcome),
            AppPhase::Cancelled => Self::Cancelled,
            AppPhase::NeedsAttention => Self::NeedsAttention,
        }
    }
}

/// Lifecycle facts emitted to an observing shell.
///
/// These events intentionally contain no transcript text, audio, clipboard
/// contents, device names, or raw error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "phase", rename_all = "snake_case")]
pub enum EngineEvent {
    PhaseChanged(EnginePhase),
}

/// Compatibility services started alongside the engine runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineOptions {
    /// Start OSWispa's existing platform tray implementation.
    pub launch_tray: bool,
    /// Start the existing Unix-socket shortcut bridge where supported.
    pub launch_ipc: bool,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            launch_tray: true,
            launch_ipc: true,
        }
    }
}

impl EngineOptions {
    /// Options for an embedding desktop shell that owns its tray and IPC.
    pub const fn embedded() -> Self {
        Self {
            launch_tray: false,
            launch_ipc: false,
        }
    }
}

/// Handle used by a desktop shell to control and observe the engine.
pub struct EngineHandle {
    command_tx: Sender<EngineCommand>,
    event_rx: Receiver<EngineEvent>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl EngineHandle {
    /// Start the complete OSWispa runtime on a named worker thread.
    pub fn start(options: EngineOptions) -> Result<Self> {
        Self::spawn_worker(move |command_rx, event_tx| {
            crate::runtime::run_engine(options, command_rx, event_tx)
        })
    }

    fn spawn_worker<F>(worker: F) -> Result<Self>
    where
        F: FnOnce(Receiver<EngineCommand>, Sender<EngineEvent>) -> Result<()> + Send + 'static,
    {
        let (command_tx, command_rx) = bounded(32);
        let (event_tx, event_rx) = unbounded();
        let terminal_event_tx = event_tx.clone();
        let worker = std::thread::Builder::new()
            .name("oswispa-engine".to_string())
            .spawn(move || {
                let result = worker(command_rx, event_tx);
                if result.is_err() {
                    let _ = terminal_event_tx
                        .send(EngineEvent::PhaseChanged(EnginePhase::NeedsAttention));
                }
                let _ = terminal_event_tx.send(EngineEvent::PhaseChanged(EnginePhase::Stopped));
                result
            })
            .context("Failed to start the OSWispa engine thread")?;

        Ok(Self {
            command_tx,
            event_rx,
            worker: Some(worker),
        })
    }

    /// Return a clonable receiver for transcript-free lifecycle events.
    pub fn events(&self) -> Receiver<EngineEvent> {
        self.event_rx.clone()
    }

    /// Send a typed engine command without blocking the caller's UI thread.
    pub fn command(&self, command: EngineCommand) -> Result<()> {
        match self.command_tx.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(anyhow!("OSWispa engine command queue is full")),
            Err(TrySendError::Disconnected(_)) => {
                Err(anyhow!("OSWispa engine is not accepting commands"))
            }
        }
    }

    pub fn start_recording(&self) -> Result<()> {
        self.command(EngineCommand::Start)
    }

    pub fn stop_recording(&self) -> Result<()> {
        self.command(EngineCommand::Stop)
    }

    pub fn cancel_recording(&self) -> Result<()> {
        self.command(EngineCommand::Cancel)
    }

    pub fn reload_config(&self) -> Result<()> {
        self.command(EngineCommand::Reload)
    }

    /// Ask the engine to stop and wait for its owning runtime thread.
    pub fn shutdown(mut self) -> Result<()> {
        // If the command side has already closed, joining still returns the
        // authoritative worker result rather than masking it with a send error.
        let _ = self.command(EngineCommand::Shutdown);
        self.join_worker()
    }

    /// Wait until the compatibility tray, IPC bridge, or another controller
    /// requests shutdown.
    pub fn wait(mut self) -> Result<()> {
        self.join_worker()
    }

    fn join_worker(&mut self) -> Result<()> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker
            .join()
            .map_err(|_| anyhow!("OSWispa engine thread panicked"))?
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.command_tx.try_send(EngineCommand::Shutdown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_handle(observed_tx: Sender<EngineCommand>) -> EngineHandle {
        EngineHandle::spawn_worker(move |command_rx, event_tx| {
            event_tx.send(EngineEvent::PhaseChanged(EnginePhase::Ready))?;
            for command in command_rx {
                observed_tx.send(command)?;
                if command == EngineCommand::Shutdown {
                    break;
                }
            }
            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn handle_forwards_typed_commands() {
        let (observed_tx, observed_rx) = unbounded();
        let handle = test_handle(observed_tx);

        handle.start_recording().unwrap();
        handle.stop_recording().unwrap();
        handle.cancel_recording().unwrap();
        handle.reload_config().unwrap();

        assert_eq!(observed_rx.recv().unwrap(), EngineCommand::Start);
        assert_eq!(observed_rx.recv().unwrap(), EngineCommand::Stop);
        assert_eq!(observed_rx.recv().unwrap(), EngineCommand::Cancel);
        assert_eq!(observed_rx.recv().unwrap(), EngineCommand::Reload);
        handle.shutdown().unwrap();
        assert_eq!(observed_rx.recv().unwrap(), EngineCommand::Shutdown);
    }

    #[test]
    fn shutdown_joins_worker_and_reports_stopped() {
        let (observed_tx, _observed_rx) = unbounded();
        let handle = test_handle(observed_tx);
        let events = handle.events();

        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            EngineEvent::PhaseChanged(EnginePhase::Ready)
        );
        handle.shutdown().unwrap();
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            EngineEvent::PhaseChanged(EnginePhase::Stopped)
        );
    }

    #[test]
    fn public_phase_redacts_listening_device_name() {
        let phase = AppPhase::Listening {
            device_name: "Private microphone label".to_string(),
        };
        let event = EngineEvent::PhaseChanged(EnginePhase::from(&phase));
        let json = serde_json::to_string(&event).unwrap();

        assert_eq!(event, EngineEvent::PhaseChanged(EnginePhase::Listening));
        assert!(!json.contains("Private microphone label"));
    }
}
