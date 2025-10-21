//! Entry point for the native wallpaper host. The real implementation will
//! bridge Win32 window hosting, DXGI swap chains, and Media Foundation playback
//! while communicating with the Tauri UI process. For now, we focus on
//! initialising tracing and verifying that the crate links correctly on CI.

use anyhow::Result;
use common::{GpuPreference, RuntimeConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    init_tracing();

    let config = RuntimeConfig {
        adapter_preference: GpuPreference::Auto,
        ..RuntimeConfig::default()
    };

    info!(?config, "wallpaper host bootstrap complete");

    // Placeholder loop until the full Direct3D + Media Foundation pipeline is wired in.
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
