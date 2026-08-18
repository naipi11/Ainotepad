use std::path::{Path, PathBuf};

use aitext_core::IndentSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    Dark,
    Light,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub font_family: String,
    pub font_size: f32,
    pub theme: ThemeName,
    pub word_wrap: bool,
    #[serde(default)]
    pub use_tabs: bool,
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    pub ghost_enabled: bool,
    pub debounce_ms: u64,
    pub ghost_color: [u8; 4],
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    pub allow_http: bool,
    pub recent_files: Vec<String>,
}

impl AppConfig {
    pub fn indent(&self) -> IndentSettings {
        IndentSettings {
            use_tabs: self.use_tabs,
            width: self.tab_width,
        }
    }

    pub fn clamped(mut self) -> Self {
        self.debounce_ms = self.debounce_ms.clamp(100, 800);
        if self.timeout_ms < 1000 {
            self.timeout_ms = 1000;
        }
        if self.tab_width == 0 {
            self.tab_width = 4;
        }
        self
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_family: "Consolas".into(),
            font_size: 14.0,
            theme: ThemeName::Dark,
            word_wrap: false,
            use_tabs: false,
            tab_width: 4,
            ghost_enabled: true,
            debounce_ms: 250,
            ghost_color: [160, 160, 160, 180],
            base_url: String::new(),
            model: String::new(),
            timeout_ms: 8000,
            allow_http: false,
            recent_files: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Parse(String),
}

fn default_tab_width() -> usize {
    4
}

pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AITEXT_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Aitext")
}

pub fn load_config() -> AppConfig {
    let path = config_dir().join("config.toml");
    match std::fs::read_to_string(path) {
        Ok(raw) => toml::from_str::<AppConfig>(&raw)
            .unwrap_or_default()
            .clamped(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| ConfigError::Io(e.to_string()))?;
    let raw = toml::to_string_pretty(config).map_err(|e| ConfigError::Parse(e.to_string()))?;
    std::fs::write(dir.join("config.toml"), raw).map_err(|e| ConfigError::Io(e.to_string()))?;
    Ok(())
}

pub fn remember_recent(config: &mut AppConfig, path: &str) {
    config.recent_files.retain(|p| p != path);
    config.recent_files.insert(0, path.to_string());
    config.recent_files.truncate(10);
}

#[allow(dead_code)]
fn ensure_parent(path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn isolated() {
        let dir = std::env::temp_dir().join(format!(
            "aitext-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
    }

    #[test]
    fn save_round_trip_does_not_write_api_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        let mut cfg = AppConfig::default();
        cfg.model = "gpt-test".into();
        cfg.base_url = "https://example.com/v1".into();
        remember_recent(&mut cfg, "C:\\tmp\\a.rs");
        save_config(&cfg).unwrap();
        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        assert!(!raw.contains("api_key"));
        assert!(raw.contains("gpt-test"));
        let loaded = load_config();
        assert_eq!(loaded.model, "gpt-test");
        assert_eq!(loaded.recent_files, vec!["C:\\tmp\\a.rs".to_string()]);
    }

    #[test]
    fn debounce_is_clamped() {
        let mut cfg = AppConfig::default();
        cfg.debounce_ms = 10;
        cfg = cfg.clamped();
        assert_eq!(cfg.debounce_ms, 100);
        cfg.debounce_ms = 9000;
        cfg = cfg.clamped();
        assert_eq!(cfg.debounce_ms, 800);
    }

    #[test]
    fn recent_files_cap_at_ten_and_move_to_front() {
        let mut cfg = AppConfig::default();
        for i in 0..12 {
            remember_recent(&mut cfg, &format!("f{i}.txt"));
        }
        assert_eq!(cfg.recent_files.len(), 10);
        assert_eq!(cfg.recent_files[0], "f11.txt");
        remember_recent(&mut cfg, "f5.txt");
        assert_eq!(cfg.recent_files[0], "f5.txt");
        assert_eq!(
            cfg.recent_files.iter().filter(|p| *p == "f5.txt").count(),
            1
        );
    }
}
