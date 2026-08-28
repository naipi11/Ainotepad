use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use aitext_ai::{AdapterKind, ProviderKind};
use aitext_core::IndentSettings;
use serde::{Deserialize, Serialize};

use crate::i18n::UiLanguage;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[serde(alias = "light")]
    #[default]
    White,
    BlackGreen,
    #[serde(rename = "vscode")]
    VsCode,
    #[serde(rename = "macos")]
    MacOs,
    Lamp,
    Dark,
    HighContrast,
    Custom,
}

impl ThemeName {
    pub fn all() -> [Self; 8] {
        [
            Self::White,
            Self::BlackGreen,
            Self::VsCode,
            Self::MacOs,
            Self::Dark,
            Self::Lamp,
            Self::HighContrast,
            Self::Custom,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::BlackGreen => "Black Green",
            Self::VsCode => "VS Code Dark",
            Self::MacOs => "macOS Light",
            Self::Lamp => "Lamp paper",
            Self::Dark => "Dark",
            Self::HighContrast => "High contrast",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomTheme {
    #[serde(default = "default_paper")]
    pub paper: [u8; 3],
    #[serde(default = "default_text")]
    pub text: [u8; 3],
    #[serde(default = "default_accent")]
    pub accent: [u8; 3],
    #[serde(default = "default_chrome")]
    pub chrome: [u8; 3],
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            paper: default_paper(),
            text: default_text(),
            accent: default_accent(),
            chrome: default_chrome(),
        }
    }
}

fn default_paper() -> [u8; 3] {
    [28, 24, 20]
}
fn default_text() -> [u8; 3] {
    [236, 226, 208]
}
fn default_accent() -> [u8; 3] {
    [214, 122, 52]
}
fn default_chrome() -> [u8; 3] {
    [22, 20, 18]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusItem {
    Cursor,
    Encoding,
    Newline,
    Language,
    Model,
    Completion,
    Message,
    Custom,
}

impl StatusItem {
    pub fn all() -> [Self; 8] {
        [
            Self::Cursor,
            Self::Encoding,
            Self::Newline,
            Self::Language,
            Self::Model,
            Self::Completion,
            Self::Message,
            Self::Custom,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::Encoding => "Encoding",
            Self::Newline => "Newline",
            Self::Language => "Language",
            Self::Model => "Model",
            Self::Completion => "Completion",
            Self::Message => "Message",
            Self::Custom => "Custom",
        }
    }
}

fn default_status_items() -> Vec<StatusItem> {
    vec![
        StatusItem::Cursor,
        StatusItem::Encoding,
        StatusItem::Newline,
        StatusItem::Language,
        StatusItem::Model,
        StatusItem::Completion,
        StatusItem::Message,
    ]
}

fn default_font_family() -> String {
    "YaHei".into()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_tab_width() -> usize {
    4
}

fn default_debounce_ms() -> u64 {
    60
}

fn default_ghost_enabled() -> bool {
    true
}

fn default_ghost_color() -> [u8; 4] {
    [160, 160, 160, 180]
}

fn default_timeout_ms() -> u64 {
    8000
}

const MAX_KNOWN_MODELS: usize = 24;
static PROFILE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn new_profile_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = PROFILE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("profile-{timestamp:x}-{counter:x}")
}

fn unique_profile_id(existing: &HashSet<String>) -> String {
    loop {
        let id = new_profile_id();
        if !existing.contains(&id) {
            return id;
        }
    }
}

fn default_profile_name() -> String {
    "API profile".into()
}

fn normalize_models(models: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    models
        .into_iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty() && seen.insert(model.clone()))
        .take(MAX_KNOWN_MODELS)
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiProfile {
    #[serde(default = "new_profile_id")]
    pub id: String,
    #[serde(default = "default_profile_name")]
    pub name: String,
    #[serde(default)]
    pub provider: ProviderKind,
    #[serde(default)]
    pub adapter: AdapterKind,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub selected_model: String,
    #[serde(default)]
    pub known_models: Vec<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub allow_http: bool,
}

impl ApiProfile {
    pub fn new(name: impl Into<String>, provider: ProviderKind) -> Self {
        let adapter = provider.default_adapter();
        Self {
            id: new_profile_id(),
            name: name.into(),
            provider,
            adapter,
            base_url: String::new(),
            selected_model: String::new(),
            known_models: Vec::new(),
            timeout_ms: default_timeout_ms(),
            allow_http: false,
        }
        .clamped()
    }

    pub fn clamped(mut self) -> Self {
        if self.provider == ProviderKind::DeepSeekFim {
            self.provider = ProviderKind::DeepSeek;
            self.adapter = AdapterKind::Fim;
        }
        self.id = self.id.trim().to_string();
        if self.id.is_empty() {
            self.id = new_profile_id();
        }
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            self.name = default_profile_name();
        }
        self.base_url = self.base_url.trim().to_string();
        self.selected_model = self.selected_model.trim().to_string();
        self.known_models = normalize_models(std::mem::take(&mut self.known_models));
        if !self.selected_model.is_empty() && !self.known_models.contains(&self.selected_model) {
            self.known_models.insert(0, self.selected_model.clone());
            self.known_models.truncate(MAX_KNOWN_MODELS);
        }
        self.timeout_ms = self.timeout_ms.clamp(1000, 30000);
        self
    }

    pub fn remember_model(&mut self, model: &str) {
        let model = model.trim();
        if model.is_empty() {
            return;
        }
        self.known_models.retain(|existing| existing != model);
        self.known_models.insert(0, model.to_string());
        self.known_models.truncate(MAX_KNOWN_MODELS);
        self.selected_model = model.to_string();
    }
}

impl Default for ApiProfile {
    fn default() -> Self {
        Self::new(default_profile_name(), ProviderKind::Custom)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct LegacyAiFields {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    known_models: Vec<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    allow_http: Option<bool>,
}

impl LegacyAiFields {
    fn has_profile_data(&self) -> bool {
        !self.base_url.trim().is_empty()
            || !self.model.trim().is_empty()
            || self
                .known_models
                .iter()
                .any(|model| !model.trim().is_empty())
    }

    fn into_profile(self) -> ApiProfile {
        let is_deepseek = url_host(&self.base_url)
            .map(|host| host.eq_ignore_ascii_case("api.deepseek.com"))
            .unwrap_or(false);
        let mut profile = ApiProfile::new(
            if is_deepseek {
                "Imported DeepSeek"
            } else {
                "Imported API"
            },
            if is_deepseek {
                ProviderKind::DeepSeek
            } else {
                ProviderKind::Custom
            },
        );
        profile.base_url = self.base_url;
        profile.selected_model = self.model;
        profile.known_models = self.known_models;
        profile.timeout_ms = self.timeout_ms.unwrap_or_else(default_timeout_ms);
        profile.allow_http = self.allow_http.unwrap_or(false);
        profile.clamped()
    }
}

fn url_host(base_url: &str) -> Option<&str> {
    let without_scheme = base_url
        .trim()
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(base_url.trim());
    let authority = without_scheme.split('/').next()?.rsplit('@').next()?;
    let host = authority.split(':').next()?.trim();
    (!host.is_empty()).then_some(host)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    pub theme: ThemeName,
    #[serde(default)]
    pub ui_language: UiLanguage,
    #[serde(default)]
    pub custom_theme: CustomTheme,
    #[serde(default)]
    pub profiles: Vec<ApiProfile>,
    #[serde(default)]
    pub active_profile_id: Option<String>,
    #[serde(default = "default_status_items")]
    pub status_items: Vec<StatusItem>,
    #[serde(default)]
    pub status_custom: String,
    pub word_wrap: bool,
    #[serde(default)]
    pub use_tabs: bool,
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    #[serde(default = "default_ghost_enabled")]
    pub ghost_enabled: bool,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_ghost_color")]
    pub ghost_color: [u8; 4],
    pub recent_files: Vec<String>,
    #[serde(default, flatten, skip_serializing)]
    legacy_ai: LegacyAiFields,
}

impl AppConfig {
    pub fn indent(&self) -> IndentSettings {
        IndentSettings {
            use_tabs: self.use_tabs,
            width: self.tab_width,
        }
    }

    pub fn clamped(mut self) -> Self {
        self.debounce_ms = self.debounce_ms.clamp(30, 800);
        if self.tab_width == 0 {
            self.tab_width = 4;
        }
        self.font_size = self.font_size.clamp(10.0, 28.0);
        if self.font_family.trim().is_empty() {
            self.font_family = default_font_family();
        }
        if self.status_items.is_empty() {
            self.status_items = default_status_items();
        }

        let mut profile_ids = HashSet::new();
        let mut profiles = Vec::with_capacity(self.profiles.len());
        for raw_profile in std::mem::take(&mut self.profiles) {
            let mut profile = raw_profile.clamped();
            if !profile_ids.insert(profile.id.clone()) {
                profile.id = unique_profile_id(&profile_ids);
                profile_ids.insert(profile.id.clone());
            }
            profiles.push(profile);
        }

        if profiles.is_empty() && self.legacy_ai.has_profile_data() {
            let profile = std::mem::take(&mut self.legacy_ai).into_profile();
            self.active_profile_id = Some(profile.id.clone());
            profiles.push(profile);
        } else {
            self.legacy_ai = LegacyAiFields::default();
        }

        let active_is_valid = self
            .active_profile_id
            .as_deref()
            .is_some_and(|profile_id| profiles.iter().any(|profile| profile.id == profile_id));
        if !active_is_valid {
            self.active_profile_id = profiles.first().map(|profile| profile.id.clone());
        }
        self.profiles = profiles;
        self
    }

    pub fn active_profile(&self) -> Option<&ApiProfile> {
        let profile_id = self.active_profile_id.as_deref()?;
        self.profiles
            .iter()
            .find(|profile| profile.id == profile_id)
    }

    pub fn active_profile_mut(&mut self) -> Option<&mut ApiProfile> {
        let profile_id = self.active_profile_id.as_deref()?;
        self.profiles
            .iter_mut()
            .find(|profile| profile.id == profile_id)
    }

    pub fn set_active_profile(&mut self, profile_id: &str) -> bool {
        if self.profiles.iter().any(|profile| profile.id == profile_id) {
            self.active_profile_id = Some(profile_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn add_profile(&mut self, mut profile: ApiProfile) {
        profile = profile.clamped();
        let profile_ids: HashSet<_> = self.profiles.iter().map(|item| item.id.clone()).collect();
        if profile_ids.contains(&profile.id) {
            profile.id = unique_profile_id(&profile_ids);
        }
        self.active_profile_id = Some(profile.id.clone());
        self.profiles.push(profile);
    }

    pub fn remove_profile(&mut self, profile_id: &str) -> Option<ApiProfile> {
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.id == profile_id)?;
        let removed = self.profiles.remove(index);
        if self.active_profile_id.as_deref() == Some(profile_id) {
            self.active_profile_id = self.profiles.first().map(|profile| profile.id.clone());
        }
        Some(removed)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: 14.0,
            theme: ThemeName::White,
            ui_language: UiLanguage::System,
            custom_theme: CustomTheme::default(),
            profiles: Vec::new(),
            active_profile_id: None,
            status_items: default_status_items(),
            status_custom: String::new(),
            word_wrap: false,
            use_tabs: false,
            tab_width: 4,
            ghost_enabled: true,
            debounce_ms: 60,
            ghost_color: [160, 160, 160, 180],
            recent_files: Vec::new(),
            legacy_ai: LegacyAiFields::default(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Parse(String),
}

pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AITEXT_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("AINOTEPAD_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Aitext")
}

pub fn legacy_config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Aitext")
}

fn legacy_config_dirs() -> Vec<PathBuf> {
    if std::env::var_os("AINOTEPAD_CONFIG_DIR").is_some()
        || std::env::var_os("AITEXT_CONFIG_DIR").is_some()
    {
        return Vec::new();
    }
    let mut dirs = Vec::new();
    if let Some(app) = std::env::var_os("APPDATA") {
        let app = PathBuf::from(app);
        dirs.push(app.join("Aitext"));
        dirs.push(app.join("Ainotepad"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        dirs.push(local.join("Aitext"));
        dirs.push(local.join("Ainotepad"));
    }
    dirs.dedup();
    dirs
}

pub fn migrate_legacy_config(new_dir: &Path, legacy_dir: &Path) -> io::Result<bool> {
    if new_dir.exists() || !legacy_dir.is_dir() {
        return Ok(false);
    }
    copy_directory(legacy_dir, new_dir)?;
    Ok(true)
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            std::fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

pub(crate) fn load_config_with_legacy_import() -> (AppConfig, Option<String>) {
    let current_dir = config_dir();
    for legacy_dir in legacy_config_dirs() {
        if current_dir != legacy_dir {
            let _ = migrate_legacy_config(&current_dir, &legacy_dir);
            if current_dir.exists() {
                break;
            }
        }
    }
    let path = current_dir.join("config.toml");
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let parsed = toml::from_str::<AppConfig>(&raw).unwrap_or_default();
            let imports_legacy_profile =
                parsed.profiles.is_empty() && parsed.legacy_ai.has_profile_data();
            let config = parsed.clamped();
            let imported_profile_id = imports_legacy_profile
                .then(|| config.active_profile_id.clone())
                .flatten();
            (config, imported_profile_id)
        }
        Err(_) => (AppConfig::default(), None),
    }
}

pub fn load_config() -> AppConfig {
    load_config_with_legacy_import().0
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
    use crate::i18n::UiLanguage;
    use aitext_ai::{AdapterKind, ProviderKind};
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
        let mut profile = ApiProfile::new("Example API", ProviderKind::Custom);
        profile.base_url = "https://example.com/v1".into();
        profile.remember_model("gpt-test");
        cfg.add_profile(profile);
        remember_recent(&mut cfg, "C:\\tmp\\a.rs");
        save_config(&cfg).unwrap();
        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        assert!(!raw.contains("api_key"));
        assert!(raw.contains("gpt-test"));
        let loaded = load_config();
        assert_eq!(loaded.active_profile().unwrap().selected_model, "gpt-test");
        assert_eq!(loaded.recent_files, vec!["C:\\tmp\\a.rs".to_string()]);
        assert!(loaded
            .active_profile()
            .unwrap()
            .known_models
            .contains(&"gpt-test".to_string()));
    }

    #[test]
    fn debounce_is_clamped() {
        let cfg = AppConfig {
            debounce_ms: 10,
            ..AppConfig::default()
        }
        .clamped();
        assert_eq!(cfg.debounce_ms, 30);
        let cfg = AppConfig {
            debounce_ms: 9000,
            ..AppConfig::default()
        }
        .clamped();
        assert_eq!(cfg.debounce_ms, 800);
    }

    #[test]
    fn new_config_defaults_to_white_theme() {
        assert_eq!(AppConfig::default().theme, ThemeName::White);
        assert_eq!(ThemeName::default(), ThemeName::White);
    }

    #[test]
    fn theme_catalog_contains_the_approved_presets() {
        assert_eq!(
            ThemeName::all(),
            [
                ThemeName::White,
                ThemeName::BlackGreen,
                ThemeName::VsCode,
                ThemeName::MacOs,
                ThemeName::Dark,
                ThemeName::Lamp,
                ThemeName::HighContrast,
                ThemeName::Custom,
            ]
        );
        assert_eq!(ThemeName::White.label(), "White");
        assert_eq!(ThemeName::BlackGreen.label(), "Black Green");
        assert_eq!(ThemeName::VsCode.label(), "VS Code Dark");
        assert_eq!(ThemeName::MacOs.label(), "macOS Light");
        assert_eq!(ThemeName::Dark.label(), "Dark");
    }

    #[test]
    fn legacy_light_theme_loads_and_saves_as_white() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        fs::write(config_dir().join("config.toml"), "theme = \"light\"\n").unwrap();

        let config = load_config();
        assert_eq!(config.theme, ThemeName::White);

        save_config(&config).unwrap();
        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        assert!(raw.contains("theme = \"white\""));
        assert!(!raw.contains("theme = \"light\""));
    }

    #[test]
    fn missing_language_defaults_to_system_without_changing_other_fields() {
        let config: AppConfig = toml::from_str(
            r#"
theme = "vscode"
font_size = 18.0
"#,
        )
        .unwrap();
        assert_eq!(config.ui_language, UiLanguage::System);
        assert_eq!(config.theme, ThemeName::VsCode);
        assert_eq!(config.font_size, 18.0);
    }

    #[test]
    fn unknown_future_language_falls_back_to_system() {
        let config: AppConfig = toml::from_str(
            r#"
ui_language = "future_locale"
theme = "dark"
"#,
        )
        .unwrap();
        assert_eq!(config.ui_language, UiLanguage::System);
        assert_eq!(config.theme, ThemeName::Dark);
    }

    #[test]
    fn language_round_trips_through_config_toml() {
        let config = AppConfig {
            ui_language: UiLanguage::ZhCn,
            ..AppConfig::default()
        };
        let raw = toml::to_string(&config).unwrap();
        let loaded: AppConfig = toml::from_str(&raw).unwrap();
        assert_eq!(loaded.ui_language, UiLanguage::ZhCn);
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

    #[test]
    fn profile_remember_model_keeps_latest_first() {
        let mut profile = ApiProfile::new("DeepSeek", ProviderKind::DeepSeek);
        profile.remember_model("deepseek-v4-flash");
        profile.remember_model("deepseek-v4-pro");
        assert_eq!(profile.selected_model, "deepseek-v4-pro");
        assert_eq!(profile.known_models[0], "deepseek-v4-pro");
        assert_eq!(
            profile
                .known_models
                .iter()
                .filter(|m| *m == "deepseek-v4-pro")
                .count(),
            1
        );
    }

    #[test]
    fn profile_selection_and_removal_keep_active_profile_valid() {
        let mut config = AppConfig::default();
        let first = ApiProfile::new("DeepSeek", ProviderKind::DeepSeek);
        let first_id = first.id.clone();
        config.add_profile(first);
        let second = ApiProfile::new("OpenAI", ProviderKind::OpenAi);
        let second_id = second.id.clone();
        config.add_profile(second);

        assert_eq!(
            config.active_profile_id.as_deref(),
            Some(second_id.as_str())
        );
        assert!(config.set_active_profile(&first_id));
        config.active_profile_mut().unwrap().base_url = "https://api.deepseek.com".into();
        assert_eq!(
            config.active_profile().unwrap().base_url,
            "https://api.deepseek.com"
        );

        let removed = config.remove_profile(&first_id).unwrap();
        assert_eq!(removed.id, first_id);
        assert_eq!(
            config.active_profile_id.as_deref(),
            Some(second_id.as_str())
        );
        assert!(!config.set_active_profile("missing"));
    }

    #[test]
    fn legacy_deepseek_config_becomes_imported_profile() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        fs::write(
            config_dir().join("config.toml"),
            r#"
 base_url = "https://api.deepseek.com"
 model = "deepseek-v4-flash"
 known_models = ["deepseek-v4-flash", "deepseek-v4-pro"]
timeout_ms = 4200
allow_http = true
"#,
        )
        .unwrap();

        let config = load_config();
        let profile = config
            .active_profile()
            .expect("legacy profile should be active");

        assert_eq!(profile.name, "Imported DeepSeek");
        assert_eq!(profile.provider, ProviderKind::DeepSeek);
        assert_eq!(profile.adapter, AdapterKind::Fim);
        assert_eq!(profile.selected_model, "deepseek-v4-flash");
        assert_eq!(profile.timeout_ms, 4200);
        assert!(profile.allow_http);
        assert_eq!(
            config.active_profile_id.as_deref(),
            Some(profile.id.as_str())
        );
    }

    #[test]
    fn old_deep_seek_fim_profile_migrates_to_deepseek_fim_adapter() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        fs::write(
            config_dir().join("config.toml"),
            r#"
active_profile_id = "legacy-deepseek"

[[profiles]]
id = "legacy-deepseek"
name = "DeepSeek"
provider = "deep_seek_fim"
adapter = "chat_completions"
base_url = "https://api.deepseek.com"
selected_model = "deepseek-v4-flash"
known_models = ["deepseek-v4-flash"]
timeout_ms = 8000
allow_http = false
"#,
        )
        .unwrap();

        let config = load_config();
        let profile = config.active_profile().unwrap();

        assert_eq!(profile.provider, ProviderKind::DeepSeek);
        assert_eq!(profile.adapter, AdapterKind::Fim);

        save_config(&config).unwrap();
        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        assert!(raw.contains("provider = \"deep_seek\""));
        assert!(raw.contains("adapter = \"fim\""));
        assert!(!raw.contains("provider = \"deep_seek_fim\""));
    }

    #[test]
    fn legacy_generic_config_becomes_custom_profile() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        fs::write(
            config_dir().join("config.toml"),
            r#"
 base_url = "https://relay.example.test/v1"
 model = "relay-fast"
 known_models = ["relay-fast"]
timeout_ms = 6500
allow_http = false
"#,
        )
        .unwrap();

        let config = load_config();
        let profile = config
            .active_profile()
            .expect("legacy profile should be active");

        assert_eq!(profile.name, "Imported API");
        assert_eq!(profile.provider, ProviderKind::Custom);
        assert_eq!(profile.adapter, AdapterKind::ChatCompletions);
        assert_eq!(profile.selected_model, "relay-fast");
        assert_eq!(profile.timeout_ms, 6500);
        assert!(!profile.allow_http);
    }

    #[test]
    fn old_openai_compatible_profile_loads_as_custom_chat_adapter() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        fs::write(
            config_dir().join("config.toml"),
            r#"
active_profile_id = "legacy-relay"

[[profiles]]
id = "legacy-relay"
name = "Old relay"
provider = "open_ai_compatible"
base_url = "https://relay.example.test/v1"
selected_model = "relay-model"
known_models = ["relay-model"]
timeout_ms = 8000
allow_http = false
"#,
        )
        .unwrap();

        let config = load_config();
        let profile = config.active_profile().unwrap();

        assert_eq!(profile.provider, ProviderKind::Custom);
        assert_eq!(profile.adapter, AdapterKind::ChatCompletions);

        save_config(&config).unwrap();
        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        assert!(raw.contains("provider = \"custom\""));
        assert!(raw.contains("adapter = \"chat_completions\""));
        assert!(!raw.contains("open_ai_compatible"));
    }

    #[test]
    fn saving_profiles_omits_legacy_top_level_ai_fields() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        isolated();
        let mut config = AppConfig::default();
        let mut profile = ApiProfile::new("OpenAI", ProviderKind::OpenAi);
        profile.base_url = "https://api.openai.com/v1".into();
        profile.remember_model("gpt-test");
        profile.timeout_ms = 9000;
        config.add_profile(profile);

        save_config(&config).unwrap();

        let raw = fs::read_to_string(config_dir().join("config.toml")).unwrap();
        let root: toml::Value = toml::from_str(&raw).unwrap();
        let table = root.as_table().unwrap();
        assert!(table.get("profiles").is_some());
        assert!(table.get("base_url").is_none());
        assert!(table.get("model").is_none());
        assert!(table.get("known_models").is_none());
        assert!(!raw.contains("test-secret-key"));
    }

    #[test]
    fn default_config_namespace_is_aitext() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let old_aitext = std::env::var_os("AITEXT_CONFIG_DIR");
        let old_ainotepad = std::env::var_os("AINOTEPAD_CONFIG_DIR");
        let old_local = std::env::var_os("LOCALAPPDATA");
        let old_app = std::env::var_os("APPDATA");
        let root =
            std::env::temp_dir().join(format!("aitext-config-namespace-{}", std::process::id()));
        std::env::remove_var("AITEXT_CONFIG_DIR");
        std::env::remove_var("AINOTEPAD_CONFIG_DIR");
        std::env::set_var("LOCALAPPDATA", &root);
        std::env::remove_var("APPDATA");

        assert_eq!(config_dir(), root.join("Aitext"));

        match old_aitext {
            Some(value) => std::env::set_var("AITEXT_CONFIG_DIR", value),
            None => std::env::remove_var("AITEXT_CONFIG_DIR"),
        }
        match old_ainotepad {
            Some(value) => std::env::set_var("AINOTEPAD_CONFIG_DIR", value),
            None => std::env::remove_var("AINOTEPAD_CONFIG_DIR"),
        }
        match old_local {
            Some(value) => std::env::set_var("LOCALAPPDATA", value),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
        match old_app {
            Some(value) => std::env::set_var("APPDATA", value),
            None => std::env::remove_var("APPDATA"),
        }
    }

    #[test]
    fn previous_ainotepad_config_migrates_into_aitext_namespace() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let old_aitext = std::env::var_os("AITEXT_CONFIG_DIR");
        let old_ainotepad = std::env::var_os("AINOTEPAD_CONFIG_DIR");
        let old_local = std::env::var_os("LOCALAPPDATA");
        let old_app = std::env::var_os("APPDATA");
        let root =
            std::env::temp_dir().join(format!("aitext-config-migration-{}", std::process::id()));
        let old_dir = root.join("Ainotepad");
        let new_dir = root.join("Aitext");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&old_dir).unwrap();
        fs::write(old_dir.join("config.toml"), "font_size = 18.0\n").unwrap();
        std::env::remove_var("AITEXT_CONFIG_DIR");
        std::env::remove_var("AINOTEPAD_CONFIG_DIR");
        std::env::set_var("LOCALAPPDATA", &root);
        std::env::remove_var("APPDATA");

        let config = load_config_with_legacy_import().0;

        assert_eq!(config.font_size, 18.0);
        assert!(new_dir.join("config.toml").exists());
        assert!(old_dir.join("config.toml").exists());

        let _ = fs::remove_dir_all(&root);
        match old_aitext {
            Some(value) => std::env::set_var("AITEXT_CONFIG_DIR", value),
            None => std::env::remove_var("AITEXT_CONFIG_DIR"),
        }
        match old_ainotepad {
            Some(value) => std::env::set_var("AINOTEPAD_CONFIG_DIR", value),
            None => std::env::remove_var("AINOTEPAD_CONFIG_DIR"),
        }
        match old_local {
            Some(value) => std::env::set_var("LOCALAPPDATA", value),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
        match old_app {
            Some(value) => std::env::set_var("APPDATA", value),
            None => std::env::remove_var("APPDATA"),
        }
    }

    #[test]
    fn legacy_config_is_copied_once_without_deleting_source() {
        let root = std::env::temp_dir().join(format!(
            "aitext-migration-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let legacy = root.join("Ainotepad");
        let new = root.join("Aitext");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("config.toml"), "theme = \"white\"\n").unwrap();

        assert!(migrate_legacy_config(&new, &legacy).unwrap());
        assert_eq!(
            fs::read_to_string(new.join("config.toml")).unwrap(),
            "theme = \"white\"\n"
        );
        assert!(legacy.join("config.toml").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn existing_aitext_config_wins_over_legacy_directory() {
        let root = std::env::temp_dir().join(format!(
            "aitext-migration-existing-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let legacy = root.join("Ainotepad");
        let new = root.join("Aitext");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(legacy.join("config.toml"), "theme = \"dark\"\n").unwrap();

        assert!(!migrate_legacy_config(&new, &legacy).unwrap());
        assert!(!new.join("config.toml").exists());
        let _ = fs::remove_dir_all(root);
    }
}
