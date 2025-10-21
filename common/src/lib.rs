//! Shared types and configuration primitives for the Bizhi wallpaper system.
//! This crate intentionally keeps runtime logic light to allow the host and UI
//! processes to depend on a small, allocation-friendly surface area.

use serde::{Deserialize, Serialize};

/// Video playback strategies supported by the wallpaper host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoScalingMode {
    /// Stretch the video to fill the monitor bounds without preserving aspect ratio.
    Stretch,
    /// Preserve aspect ratio and crop overflow to cover the entire monitor.
    Cover,
    /// Preserve aspect ratio and letterbox/pillarbox to fit entirely on screen.
    Contain,
}

impl Default for VideoScalingMode {
    fn default() -> Self {
        Self::Cover
    }
}

/// Basic runtime configuration shared between the host and the UI facade.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RuntimeConfig {
    /// Preferred GPU selection strategy.
    pub adapter_preference: GpuPreference,
    /// Playback scaling behaviour.
    pub scaling_mode: VideoScalingMode,
    /// Optional path to the active wallpaper asset.
    pub active_media: Option<String>,
}

/// Host-side GPU preference mapping for DXGI 1.6 enumeration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPreference {
    /// Request the minimum power GPU (usually the integrated GPU).
    MinimumPower,
    /// Request the high performance GPU (typically a discrete GPU).
    HighPerformance,
    /// Allow the runtime to decide dynamically based on heuristics.
    Auto,
}

impl Default for GpuPreference {
    fn default() -> Self {
        Self::Auto
    }
}
