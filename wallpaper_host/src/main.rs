//! Entry point for the native wallpaper host. The real implementation will
//! bridge Win32 window hosting, DXGI swap chains, and Media Foundation playback
//! while communicating with the Tauri UI process. For now, we focus on
//! initialising tracing and verifying that the crate links correctly on CI.

use anyhow::{Context, Result};
use common::{GpuPreference, RuntimeConfig, VideoScalingMode};
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    init_tracing();

    let config = RuntimeConfig {
        adapter_preference: GpuPreference::Auto,
        ..RuntimeConfig::default()
    };

    let mut state = HostState::new(config);

    info!(config = ?state.config, "wallpaper host bootstrap complete");
    info!("type 'help' for an overview of the available commands");

    run_cli(&mut state)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PlaybackState {
    Playing,
    Paused,
}

impl fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Playing => f.write_str("playing"),
            Self::Paused => f.write_str("paused"),
        }
    }
}

#[derive(Debug)]
struct HostState {
    config: RuntimeConfig,
    playback_state: PlaybackState,
}

impl HostState {
    fn new(config: RuntimeConfig) -> Self {
        Self {
            config,
            playback_state: PlaybackState::Paused,
        }
    }

    fn load_media(&mut self, path: &str) {
        let canonical = PathBuf::from(path);
        self.config.active_media = Some(canonical.to_string_lossy().into_owned());
        self.playback_state = PlaybackState::Playing;
        info!(media = ?self.config.active_media, "queued new wallpaper media");
    }

    fn clear_media(&mut self) -> bool {
        if self.config.active_media.take().is_some() {
            self.playback_state = PlaybackState::Paused;
            info!("cleared active wallpaper media");
            true
        } else {
            false
        }
    }

    fn pause(&mut self) -> bool {
        if self.playback_state != PlaybackState::Paused {
            self.playback_state = PlaybackState::Paused;
            info!("paused wallpaper playback");
            true
        } else {
            false
        }
    }

    fn resume(&mut self) -> bool {
        if self.playback_state != PlaybackState::Playing {
            self.playback_state = PlaybackState::Playing;
            info!("resumed wallpaper playback");
            true
        } else {
            false
        }
    }

    fn set_scaling(&mut self, mode: VideoScalingMode) -> bool {
        if self.config.scaling_mode != mode {
            self.config.scaling_mode = mode;
            info!(%mode, "updated scaling mode");
            true
        } else {
            false
        }
    }
}

fn run_cli(state: &mut HostState) -> Result<()> {
    print_help();
    let stdin = io::stdin();

    loop {
        print!("bizhi> ");
        io::stdout().flush().context("failed to flush prompt")?;

        let mut line = String::new();
        if stdin
            .read_line(&mut line)
            .context("failed to read command input")?
            == 0
        {
            println!();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (raw_cmd, rest) = trimmed
            .split_once(char::is_whitespace)
            .map_or((trimmed, ""), |(cmd, tail)| (cmd, tail.trim_start()));
        let cmd = raw_cmd.to_ascii_lowercase();

        match cmd.as_str() {
            "help" | "?" => print_help(),
            "status" => print_status(state),
            "open" | "load" => {
                if rest.is_empty() {
                    println!("usage: open <path-to-video>");
                    continue;
                }
                state.load_media(rest);
                println!("queued video: {rest}");
            }
            "clear" => {
                if state.clear_media() {
                    println!("cleared active media");
                } else {
                    println!("no active media loaded");
                }
            }
            "pause" => {
                if state.pause() {
                    println!("playback paused");
                } else {
                    println!("playback already paused");
                }
            }
            "resume" | "play" => {
                if state.resume() {
                    println!("playback resumed");
                } else {
                    println!("playback already running");
                }
            }
            "mode" => {
                if rest.is_empty() {
                    println!("usage: mode <stretch|cover|contain>");
                    continue;
                }
                match rest.parse::<VideoScalingMode>() {
                    Ok(mode) => {
                        if state.set_scaling(mode) {
                            println!("scaling mode set to {mode}");
                        } else {
                            println!("scaling mode already set to {mode}");
                        }
                    }
                    Err(err) => {
                        error!(%err, "failed to parse scaling mode");
                        println!("{err}");
                    }
                }
            }
            "quit" | "exit" => {
                println!("exiting wallpaper host stub");
                break;
            }
            other => {
                println!("unknown command '{other}'. type 'help' for options");
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("  help                Show this help message");
    println!("  status              Print current configuration");
    println!("  open <path>         Queue a wallpaper video for playback");
    println!("  clear               Remove the active wallpaper video");
    println!("  pause               Pause playback");
    println!("  resume              Resume playback");
    println!("  mode <variant>      Change scaling (stretch | cover | contain)");
    println!("  exit                Quit the wallpaper host");
}

fn print_status(state: &HostState) {
    println!("Playback: {}", state.playback_state);
    println!("Scaling : {}", state.config.scaling_mode);
    match state.config.active_media.as_deref() {
        Some(path) => println!("Media   : {path}"),
        None => println!("Media   : <none>"),
    }
}
