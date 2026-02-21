//! Configuration file management for gittui.
//!
//! Reads and writes `~/.config/gittui/config.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub theme: ThemeConfig,
    pub diff: DiffConfig,
    pub log: LogConfig,
    pub ai: AiConfig,
}

/// General settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Refresh interval for the tick timer (milliseconds).
    pub refresh_interval_ms: u64,
    /// Whether mouse support is enabled.
    pub mouse_enabled: bool,
    /// External editor command for commit messages (optional).
    pub editor: String,
}

/// Theme settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Name of the colour theme.
    pub name: String,
}

/// Diff viewer settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    /// Number of context lines around changes.
    pub context_lines: u32,
    /// Default diff display mode: "unified" or "side-by-side".
    pub mode: String,
    /// Enable word-level diff highlighting.
    pub word_diff: bool,
}

/// Log / graph settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    /// Graph style: "rounded", "ascii", or "unicode".
    pub graph_style: String,
    /// Show remote-tracking branches in the log graph.
    pub show_remote_branches: bool,
    /// Maximum number of commits to load.
    pub max_count: usize,
}

/// AI assistant settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Whether AI features are enabled.
    pub enabled: bool,
    /// Model to use (passed as ANTHROPIC_MODEL env var).
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: String::from("haiku"),
        }
    }
}

// ── Defaults ─────────────────────────────────────────────────────────────


impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 250,
            mouse_enabled: true,
            editor: String::from("vim"),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: String::from("default"),
        }
    }
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            context_lines: 3,
            mode: String::from("unified"),
            word_diff: true,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            graph_style: String::from("rounded"),
            show_remote_branches: true,
            max_count: 500,
        }
    }
}

// ── I/O ──────────────────────────────────────────────────────────────────

/// Return the path to the configuration file.
///
/// `~/.config/gittui/config.toml` on Linux / macOS.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gittui")
        .join("config.toml")
}

/// Load the configuration, falling back to defaults if the file does not
/// exist or cannot be parsed.
pub fn load_config() -> Config {
    try_load_config().unwrap_or_default()
}

/// Attempt to load and parse the configuration file.
fn try_load_config() -> Result<Config> {
    let path = config_path();
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: Config =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(cfg)
}

/// Save the current configuration to disk.
#[allow(dead_code)]
pub fn save_config(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    let content =
        toml::to_string_pretty(cfg).context("failed to serialise config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
