use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            width: 300.0,
            height: 440.0,
        }
    }
}

impl WindowConfig {
    pub fn config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("spark")
                    .join("window_state.json"),
            );
        }
        paths.push(PathBuf::from(".spark").join("window_state.json"));
        paths
    }

    pub fn load() -> Self {
        for path in Self::config_paths() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<WindowConfig>(&content) {
                    // Ensure reasonable constraints for desktop usage
                    let width = config.width.clamp(240.0, 1600.0);
                    let height = config.height.clamp(280.0, 2000.0);
                    let x = config.x.max(0.0);
                    let y = config.y.max(0.0);
                    return Self {
                        x,
                        y,
                        width,
                        height,
                    };
                }
            }
        }

        Self::default()
    }

    pub fn save(&self) {
        let Ok(content) = serde_json::to_string_pretty(self) else {
            return;
        };

        for path in Self::config_paths() {
            if let Some(parent) = path.parent() {
                if fs::create_dir_all(parent).is_ok() {
                    if fs::write(&path, &content).is_ok() {
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.width, 300.0);
        assert_eq!(cfg.height, 440.0);
    }

    #[test]
    fn test_window_config_serde() {
        let cfg = WindowConfig {
            x: 250.0,
            y: 180.0,
            width: 360.0,
            height: 520.0,
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let parsed: WindowConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.x, 250.0);
        assert_eq!(parsed.y, 180.0);
        assert_eq!(parsed.width, 360.0);
        assert_eq!(parsed.height, 520.0);
    }
}

