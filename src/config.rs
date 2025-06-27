use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowConfig {
    pub x: i32,
    pub y: i32,
    pub width: f32,
    pub height: f32,
    pub monitor_index: Option<usize>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1200.0,
            height: 800.0,
            monitor_index: None,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppConfig {
    pub window: WindowConfig,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        
        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => eprintln!("Failed to parse config: {}", e),
                },
                Err(e) => eprintln!("Failed to read config file: {}", e),
            }
        }
        
        Self::default()
    }
    
    pub fn save(&self) {
        let config_path = Self::config_path();
        
        if let Some(parent) = config_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
        }
        
        match toml::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(e) = fs::write(&config_path, contents) {
                    eprintln!("Failed to write config file: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to serialize config: {}", e),
        }
    }
    
    fn config_path() -> PathBuf {
        PathBuf::from("./config.toml")
    }
}