//! IPC message definitions shared between the Tauri UI process and the native
//! wallpaper host. The goal is to provide a stable, serialisable contract that
//! Tauri commands can use without depending on implementation details.

use common::{GpuPreference, RuntimeConfig, VideoScalingMode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Commands initiated from the UI towards the wallpaper host.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum HostCommand {
    /// Load a new video file and start playback.
    LoadVideo { path: String },
    /// Toggle playback between paused and playing states.
    TogglePause,
    /// Force a specific playback state.
    SetPlayback { paused: bool },
    /// Change the current scaling mode.
    SetScaling { mode: VideoScalingMode },
    /// Request a GPU preference switch (recreates devices as needed).
    SetGpuPreference { preference: GpuPreference },
    /// Gracefully shut down the host process.
    Exit,
}

/// Events sent from the host back to the UI for state synchronisation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum UiEvent {
    /// Report a change in playback state.
    PlaybackState { paused: bool },
    /// Surface a recoverable error to the UI.
    RecoverableError { message: String },
    /// Push the current runtime configuration snapshot.
    RuntimeConfig { config: RuntimeConfig },
}

/// Errors that can occur while handling IPC messages.
#[derive(Debug, Error)]
pub enum IpcError {
    /// The incoming payload could not be deserialised.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    /// The request references a resource that is unavailable.
    #[error("resource unavailable: {0}")]
    ResourceUnavailable(String),
}

/// Convenience alias for fallible IPC results.
pub type IpcResult<T> = Result<T, IpcError>;
