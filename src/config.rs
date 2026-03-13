use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::models::SshConnection;

// ──────────────────────────────────────────────────────────────────────────────
// AppConfig — persisted to disk as JSON
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub connections: Vec<SshConnection>,

    // Appearance
    #[serde(default = "default_theme")]
    pub theme: String,

    // Terminal
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,

    // SSH defaults
    #[serde(default = "default_username")]
    pub default_username: String,
    #[serde(default = "default_port")]
    pub default_port: u16,

    // Local shell override (empty = use $SHELL → /bin/bash)
    #[serde(default)]
    pub local_shell: String,

    // Session logging
    #[serde(default)]
    pub log_sessions: bool,
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
}

fn default_theme()      -> String { "cyberpunk".into() }
fn default_font_size()  -> f32    { 14.0 }
fn default_scrollback() -> usize  { 5_000 }
fn default_username()   -> String { std::env::var("USER").unwrap_or_else(|_| "root".into()) }
fn default_port()       -> u16    { 22 }
fn default_log_dir()    -> String {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("shellkeeper").join("logs")
        .to_string_lossy().into_owned()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connections:      Vec::new(),
            theme:            default_theme(),
            font_size:        default_font_size(),
            scrollback_lines: default_scrollback(),
            default_username: default_username(),
            default_port:     default_port(),
            local_shell:      String::new(),
            log_sessions:     false,
            log_dir:          default_log_dir(),
        }
    }
}

impl AppConfig {
    fn path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("shellkeeper").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Self::default();
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
