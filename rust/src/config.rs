use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Parser, Debug, Clone)]
#[command(name = "sugar-wifi-conf", about = "BLE WiFi configuration service for Raspberry Pi")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// BLE advertised device name
    #[arg(long, default_value = "raspberrypi", global = true)]
    pub name: String,

    /// Security key for WiFi configuration commands
    #[arg(long, default_value = "pisugar", global = true)]
    pub key: String,

    /// Path to custom_config.json
    #[arg(long, default_value = "custom_config.json", global = true)]
    pub config: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Start BLE service (default)
    Serve,
    /// Interactive config file editor
    Config,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomConfig {
    #[serde(default)]
    pub info: Vec<InfoItem>,
    #[serde(default)]
    pub commands: Vec<CommandItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InfoItem {
    pub label: String,
    pub command: String,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandItem {
    pub label: String,
    pub command: String,
}

fn default_interval() -> u64 {
    10
}

impl CustomConfig {
    pub fn load(path: &str) -> Self {
        let p = Path::new(path);
        if !p.exists() {
            log::warn!("Config file not found: {}, using empty config", path);
            return CustomConfig {
                info: vec![],
                commands: vec![],
            };
        }
        match fs::read_to_string(p) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    log::error!("Failed to parse config: {}", e);
                    CustomConfig {
                        info: vec![],
                        commands: vec![],
                    }
                }
            },
            Err(e) => {
                log::error!("Failed to read config file: {}", e);
                CustomConfig {
                    info: vec![],
                    commands: vec![],
                }
            }
        }
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json)
            .map_err(|e| format!("Failed to write config file: {}", e))
    }
}
