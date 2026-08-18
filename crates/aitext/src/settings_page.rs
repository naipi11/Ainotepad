use crate::commands::AitextApp;
use crate::config::{save_config, ConfigError, ThemeName};
use crate::secrets::{load_api_key, store_api_key};
use egui::Ui;

impl AitextApp {
    pub fn save_settings(&mut self) -> Result<(), ConfigError> {
        self.config = self.config.clone().clamped();
        save_config(&self.config)?;
        if !self.pending_api_key.is_empty() {
            store_api_key(&self.pending_api_key).map_err(|e| ConfigError::Io(format!("{e:?}")))?;
        }
        Ok(())
    }
}

pub fn draw_settings(ui: &mut Ui, app: &mut AitextApp) {
    ui.heading("Settings");
    ui.horizontal(|ui| {
        ui.label("Font");
        ui.text_edit_singleline(&mut app.config.font_family);
        ui.add(egui::Slider::new(&mut app.config.font_size, 10.0..=28.0).text("size"));
    });
    ui.horizontal(|ui| {
        ui.label("Theme");
        ui.radio_value(&mut app.config.theme, ThemeName::Dark, "Dark");
        ui.radio_value(&mut app.config.theme, ThemeName::Light, "Light");
    });
    ui.checkbox(&mut app.config.word_wrap, "Word wrap");
    ui.checkbox(&mut app.config.use_tabs, "Use tabs");
    ui.add(egui::Slider::new(&mut app.config.tab_width, 1..=8).text("Tab width"));
    ui.separator();
    ui.checkbox(&mut app.config.ghost_enabled, "Ghost text");
    ui.add(egui::Slider::new(&mut app.config.debounce_ms, 100..=800).text("Debounce ms"));
    ui.horizontal(|ui| {
        ui.label("Base URL");
        ui.text_edit_singleline(&mut app.config.base_url);
    });
    ui.horizontal(|ui| {
        ui.label("Model");
        ui.text_edit_singleline(&mut app.config.model);
    });
    ui.horizontal(|ui| {
        ui.label("API key");
        ui.add(egui::TextEdit::singleline(&mut app.pending_api_key).password(true));
    });
    ui.add(egui::Slider::new(&mut app.config.timeout_ms, 1000..=30000).text("Timeout ms"));
    ui.checkbox(&mut app.config.allow_http, "Allow plaintext HTTP");
    if ui.button("Test connection").clicked() {
        if app.config.base_url.is_empty() || app.config.model.is_empty() || (app.pending_api_key.is_empty() && load_api_key().ok().flatten().is_none()) {
            app.status = "not configured".into();
        } else {
            app.status = "ready to test".into();
        }
    }
    if ui.button("Save").clicked() {
        match app.save_settings() {
            Ok(()) => app.status = "settings saved".into(),
            Err(err) => app.status = format!("save failed: {err:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::AitextApp;
    use crate::secrets::load_api_key;

    #[test]
    fn settings_save_writes_model_but_not_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aitext-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        let mut app = AitextApp::new_for_test();
        app.config.model = "m1".into();
        app.pending_api_key = "sk-test".into();
        app.save_settings().unwrap();
        let raw = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(raw.contains("m1"));
        assert!(!raw.contains("sk-test"));
        assert_eq!(load_api_key().unwrap().as_deref(), Some("sk-test"));
    }
}
