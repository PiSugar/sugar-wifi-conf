use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct DeviceAliases {
    /// Map from BLE device ID → user-chosen alias
    pub aliases: HashMap<String, String>,
}

impl DeviceAliases {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, data);
        }
    }

    pub fn get(&self, device_id: &str) -> Option<&str> {
        self.aliases.get(device_id).map(|s| s.as_str())
    }

    pub fn set(&mut self, device_id: &str, alias: &str) {
        self.aliases.insert(device_id.to_string(), alias.to_string());
        self.save();
    }

    pub fn remove(&mut self, device_id: &str) {
        self.aliases.remove(device_id);
        self.save();
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ble-ssh");
    path.push("aliases.json");
    path
}
