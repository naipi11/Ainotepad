use aitext_core::{Direction, FindQuery, Match, Workspace};

use aitext_ai::{CancelFlag, EngineEvent};

use crate::completion::CompletionUiState;
use crate::config::{load_config_with_legacy_import, AppConfig};
use crate::i18n::{
    resolve_locale, text, windows_user_locale_tag, Locale, TextKey, UiLanguage, UiMessage,
};

fn load_startup_config() -> AppConfig {
    let (config, imported_profile_id) = load_config_with_legacy_import();
    if let Some(profile_id) = imported_profile_id {
        let _ = crate::secrets::migrate_legacy_api_key(&profile_id);
    }
    config
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewTab,
    Open,
    Save,
    SaveAs,
    CloseTab,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Indent,
    Unindent,
    Find,
    Replace,
    Settings,
    AcceptGhost,
    RejectGhost,
    NextTab,
    PrevTab,
}

#[derive(Clone, Debug)]
pub struct FindBarState {
    pub visible: bool,
    pub replace_visible: bool,
    pub query: FindQuery,
    pub replacement: String,
    pub current: usize,
    pub matches: Vec<Match>,
}

impl Default for FindBarState {
    fn default() -> Self {
        Self {
            visible: false,
            replace_visible: false,
            query: FindQuery {
                text: String::new(),
                match_case: false,
                whole_word: false,
            },
            replacement: String::new(),
            current: 0,
            matches: Vec::new(),
        }
    }
}

pub struct AitextApp {
    pub workspace: Workspace,
    pub config: AppConfig,
    pub find: FindBarState,
    pub settings_open: bool,
    pub settings_section: crate::settings_page::SettingsSection,
    pub status: Option<UiMessage>,
    pub completion_label: String,
    pub clipboard: String,
    pub pending_api_key: String,
    pub(crate) pending_api_key_clear: bool,
    pub pending_model: String,
    pub(crate) profile_delete_pending: Option<String>,
    pub(crate) pending_profile_secret_deletions: Vec<String>,
    pub about_open: bool,
    pub shortcuts_open: bool,
    pub completion: CompletionUiState,
    pub api_key: Option<String>,
    pub profile_revision: u64,
    pub(crate) profile_worker_inboxes:
        Vec<std::sync::mpsc::Receiver<crate::settings_page::ProfileWorkerResult>>,
    pub last_engine_event: Option<EngineEvent>,
    pub ime: ImeState,
    pub(crate) system_locale: Locale,
}

impl AitextApp {
    pub fn new_for_test() -> Self {
        Self {
            workspace: Workspace::new(),
            config: AppConfig::default(),
            find: FindBarState::default(),
            settings_open: false,
            settings_section: crate::settings_page::SettingsSection::default(),
            status: None,
            completion_label: "empty".into(),
            clipboard: String::new(),
            pending_api_key: String::new(),
            pending_api_key_clear: false,
            pending_model: String::new(),
            profile_delete_pending: None,
            pending_profile_secret_deletions: Vec::new(),
            about_open: false,
            shortcuts_open: false,
            completion: CompletionUiState::default(),
            api_key: None,
            profile_revision: 0,
            profile_worker_inboxes: Vec::new(),
            last_engine_event: None,
            ime: ImeState::default(),
            system_locale: Locale::En,
        }
    }

    pub fn apply_ime(&mut self, event: egui::ImeEvent) {
        let action = self.ime.on_event(&egui::Event::Ime(event));
        self.completion.composing = self.ime.composing;
        match action {
            crate::ime::ImeAction::Commit(text) => self.handle_text_input(&text),
            crate::ime::ImeAction::PreeditChanged => {
                self.completion.engine.reject();
            }
            crate::ime::ImeAction::None => {}
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::app::install_fonts(&cc.egui_ctx);
        let mut app = Self::new_for_test();
        app.config = load_startup_config();
        app.system_locale =
            resolve_locale(UiLanguage::System, windows_user_locale_tag().as_deref());
        app.reload_active_profile_key();
        if !crate::app::font_is_available(&app.config.font_family) {
            app.config.font_family = crate::app::fallback_font_family();
        }
        app.refresh_completion_config();
        if app.workspace.current_id().is_none() {
            app.workspace.new_untitled();
        }
        app
    }

    pub fn locale(&self) -> Locale {
        match self.config.ui_language {
            UiLanguage::System => self.system_locale,
            UiLanguage::ZhCn => Locale::ZhCn,
            UiLanguage::En => Locale::En,
        }
    }

    pub fn tr(&self, key: TextKey) -> &'static str {
        text(self.locale(), key)
    }

    pub fn set_ui_language(&mut self, language: UiLanguage) {
        self.config.ui_language = language;
    }

    pub fn status_text(&self) -> Option<String> {
        self.status
            .as_ref()
            .map(|message| message.render(self.locale()))
    }

    pub fn reload_active_profile_key(&mut self) {
        self.api_key = self.config.active_profile().and_then(|profile| {
            crate::secrets::load_profile_api_key(&profile.id)
                .ok()
                .flatten()
        });
    }

    pub fn activate_profile(&mut self, profile_id: &str) -> bool {
        if !self.config.set_active_profile(profile_id) {
            return false;
        }
        self.profile_changed();
        true
    }

    pub fn profile_changed(&mut self) {
        self.pending_api_key.clear();
        self.pending_api_key_clear = false;
        self.reload_active_profile_key();
        self.profile_edited();
    }

    /// Clears the active key from memory and marks its profile-scoped secret
    /// for removal the next time settings are saved.  This deliberately does
    /// not reload the stored secret: doing so would make a cleared key usable
    /// again before the user has confirmed the save.
    pub(crate) fn clear_active_profile_api_key(&mut self) {
        self.pending_api_key.clear();
        self.api_key = None;
        self.pending_api_key_clear = self.config.active_profile().is_some();
        self.profile_edited();
    }

    pub fn remove_profile(&mut self, profile_id: &str) -> bool {
        let was_active = self.config.active_profile_id.as_deref() == Some(profile_id);
        if self.config.remove_profile(profile_id).is_none() {
            return false;
        }
        if !self
            .pending_profile_secret_deletions
            .iter()
            .any(|pending| pending == profile_id)
        {
            self.pending_profile_secret_deletions
                .push(profile_id.to_string());
        }
        self.profile_delete_pending = None;
        if was_active {
            self.profile_changed();
        } else {
            self.profile_edited();
        }
        true
    }

    /// Invalidates in-memory work after an unsaved edit to the active profile.
    /// Draft key text is intentionally retained so editing a field cannot erase
    /// the API key the user is still entering.
    pub fn profile_edited(&mut self) {
        self.completion.cancel.cancel();
        self.completion.cancel = CancelFlag::new();
        self.completion.inflight = None;
        self.completion.inbox = None;
        self.profile_worker_inboxes.clear();
        self.refresh_completion_config();
        self.completion.engine.invalidate();
        self.profile_revision = self.profile_revision.wrapping_add(1);
    }

    pub fn dispatch(&mut self, command: Command) {
        match command {
            Command::NewTab => {
                self.workspace.new_untitled();
            }
            Command::CloseTab => {
                if let Some(id) = self.workspace.current_id() {
                    if let Some(doc) = self.workspace.get(id) {
                        if doc.is_dirty() {
                            self.status = Some(UiMessage::UnsavedChanges);
                        }
                    }
                    self.workspace.close(id);
                }
            }
            Command::Undo => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.undo();
                }
                self.invalidate_and_queue();
            }
            Command::Redo => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.redo();
                }
                self.invalidate_and_queue();
            }
            Command::SelectAll => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.set_selection(aitext_core::Selection {
                        anchor: 0,
                        caret: doc.len_chars(),
                    });
                }
            }
            Command::Cut => {
                if let Some(doc) = self.workspace.current_mut() {
                    self.clipboard = doc.selected_text();
                    if !self.clipboard.is_empty() {
                        doc.insert("");
                    }
                }
                self.invalidate_and_queue();
            }
            Command::Copy => {
                if let Some(doc) = self.workspace.current() {
                    self.clipboard = doc.selected_text();
                }
            }
            Command::Paste => {
                let text = self.clipboard.clone();
                if let Some(doc) = self.workspace.current_mut() {
                    doc.insert(&text);
                }
                self.invalidate_and_queue();
            }
            Command::Indent => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.indent(self.config.indent());
                }
            }
            Command::Unindent => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.unindent(self.config.indent());
                }
            }
            Command::Find => {
                self.find.visible = true;
                self.find.replace_visible = false;
            }
            Command::Replace => {
                self.find.visible = true;
                self.find.replace_visible = true;
            }
            Command::Settings => self.settings_open = !self.settings_open,
            Command::NextTab => self.workspace.next_tab(),
            Command::PrevTab => self.workspace.prev_tab(),
            Command::AcceptGhost => self.accept_ghost(),
            Command::RejectGhost => self.reject_ghost(),
            Command::Open => self.open_file(),
            Command::Save => self.save_file(false),
            Command::SaveAs => self.save_file(true),
            Command::Exit => {}
        }
    }

    pub fn refresh_find(&mut self) {
        let Some(doc) = self.workspace.current() else {
            self.find.matches.clear();
            return;
        };
        self.find.matches = doc.find_all(&self.find.query);
        if self.find.current >= self.find.matches.len() {
            self.find.current = 0;
        }
    }

    pub fn find_step(&mut self, direction: Direction) {
        self.refresh_find();
        if self.find.matches.is_empty() {
            return;
        }
        match direction {
            Direction::Forward => {
                self.find.current = (self.find.current + 1) % self.find.matches.len();
            }
            Direction::Backward => {
                if self.find.current == 0 {
                    self.find.current = self.find.matches.len() - 1;
                } else {
                    self.find.current -= 1;
                }
            }
        }
        if let (Some(m), Some(doc)) = (
            self.find.matches.get(self.find.current).copied(),
            self.workspace.current_mut(),
        ) {
            doc.set_selection(aitext_core::Selection {
                anchor: m.start,
                caret: m.end,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Locale, TextKey, UiLanguage, UiMessage};
    use crate::secrets::{load_profile_api_key, store_api_key, store_profile_api_key};

    fn isolated_config_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aitext-commands-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn new_tab_then_insert_then_undo() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.workspace.current_mut().unwrap().insert("x");
        assert_eq!(app.workspace.current().unwrap().text(), "x");
        app.dispatch(Command::Undo);
        assert_eq!(app.workspace.current().unwrap().text(), "");
    }

    #[test]
    fn close_last_clean_tab_leaves_empty_workspace() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let id = app.workspace.current_id().unwrap();
        app.dispatch(Command::CloseTab);
        assert!(app.workspace.current_id().is_none());
        assert!(app.workspace.get(id).is_none());
    }

    #[test]
    fn tab_accepts_visible_ghost_as_one_insert() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.workspace.current_mut().unwrap().insert("he");
        app.force_ghost("llo");
        app.dispatch(Command::AcceptGhost);
        assert_eq!(app.workspace.current().unwrap().text(), "hello");
        assert!(app.completion.engine.suggestion().is_none());
        app.dispatch(Command::Undo);
        assert_eq!(app.workspace.current().unwrap().text(), "he");
    }

    #[test]
    fn typing_matching_prefix_trims_ghost() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.force_ghost("hello");
        app.handle_text_input("he");
        assert_eq!(app.completion.engine.suggestion().unwrap().text, "llo");
    }

    #[test]
    fn esc_rejects_without_editing() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.workspace.current_mut().unwrap().insert("ab");
        app.force_ghost("cd");
        app.dispatch(Command::RejectGhost);
        assert_eq!(app.workspace.current().unwrap().text(), "ab");
        assert!(app.completion.engine.suggestion().is_none());
    }

    #[test]
    fn composing_skips_requests() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let mut profile =
            crate::config::ApiProfile::new("Test API", aitext_ai::ProviderKind::Custom);
        profile.base_url = "https://example.com/v1".into();
        profile.remember_model("m");
        app.config.add_profile(profile);
        app.api_key = Some("k".into());
        app.completion.engine.configured = true;
        app.completion.composing = true;
        app.handle_text_input("你");
        assert!(!matches!(
            app.last_engine_event,
            Some(aitext_ai::EngineEvent::StartRequest { .. })
        ));
    }

    #[test]
    fn queue_completion_uses_loaded_key() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let mut profile =
            crate::config::ApiProfile::new("DeepSeek", aitext_ai::ProviderKind::DeepSeek);
        profile.base_url = "https://api.deepseek.com/v1".into();
        profile.remember_model("deepseek-v4-flash");
        app.config.add_profile(profile);
        app.api_key = Some("sk-test".into());
        app.handle_text_input("fn ");
        app.poll_completion(10_000);
        assert!(matches!(
            app.last_engine_event,
            Some(aitext_ai::EngineEvent::StartRequest { .. })
        ));
        assert_eq!(
            app.completion.engine.state(),
            aitext_ai::CompletionState::Requesting
        );
    }

    #[test]
    fn language_switch_is_immediate_and_does_not_touch_editor_or_ai_state() {
        let mut app = AitextApp::new_for_test();
        app.workspace.new_untitled();
        app.workspace.current_mut().unwrap().insert("你好");
        app.force_ghost("，世界");
        let generation = app.completion.engine.generation();
        let revision = app.profile_revision;

        app.set_ui_language(UiLanguage::ZhCn);

        assert_eq!(app.locale(), Locale::ZhCn);
        assert_eq!(app.tr(TextKey::MenuFile), "文件");
        assert_eq!(app.workspace.current().unwrap().text(), "你好");
        assert_eq!(app.ghost_text(), Some("，世界"));
        assert_eq!(app.completion.engine.generation(), generation);
        assert_eq!(app.profile_revision, revision);
    }

    #[test]
    fn system_language_uses_captured_windows_locale() {
        let mut app = AitextApp::new_for_test();
        app.system_locale = Locale::ZhCn;
        app.set_ui_language(UiLanguage::System);
        assert_eq!(app.locale(), Locale::ZhCn);
    }

    #[test]
    fn typed_status_rerenders_after_language_switch() {
        let mut app = AitextApp::new_for_test();
        app.status = Some(UiMessage::UnsavedChanges);
        app.set_ui_language(UiLanguage::En);
        assert_eq!(app.status_text().as_deref(), Some("Unsaved changes"));
        app.set_ui_language(UiLanguage::ZhCn);
        assert_eq!(app.status_text().as_deref(), Some("有未保存的更改"));
    }

    #[test]
    fn reload_active_profile_key_uses_only_the_active_profile_secret() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = isolated_config_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        let mut app = AitextApp::new_for_test();
        let first = crate::config::ApiProfile::new("DeepSeek", aitext_ai::ProviderKind::DeepSeek);
        let first_id = first.id.clone();
        app.config.add_profile(first);
        let second = crate::config::ApiProfile::new("OpenAI", aitext_ai::ProviderKind::OpenAi);
        let second_id = second.id.clone();
        app.config.add_profile(second);

        store_api_key("legacy-key-that-must-not-win").unwrap();
        store_profile_api_key(&first_id, "first-profile-key").unwrap();
        store_profile_api_key(&second_id, "second-profile-key").unwrap();

        app.reload_active_profile_key();
        assert_eq!(app.api_key.as_deref(), Some("second-profile-key"));

        assert!(app.config.set_active_profile(&first_id));
        app.reload_active_profile_key();
        assert_eq!(app.api_key.as_deref(), Some("first-profile-key"));
    }

    #[test]
    fn startup_does_not_copy_legacy_key_into_existing_profile() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = isolated_config_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        let mut config = AppConfig::default();
        let mut profile = crate::config::ApiProfile::new("OpenAI", aitext_ai::ProviderKind::OpenAi);
        profile.base_url = "https://api.openai.com/v1".into();
        profile.remember_model("gpt-test");
        let profile_id = profile.id.clone();
        config.add_profile(profile);
        crate::config::save_config(&config).unwrap();
        store_api_key("legacy-deepseek-key").unwrap();

        let loaded = load_startup_config();

        assert_eq!(
            loaded.active_profile_id.as_deref(),
            Some(profile_id.as_str())
        );
        assert_eq!(load_profile_api_key(&profile_id).unwrap(), None);
    }

    #[test]
    fn startup_copies_legacy_key_only_to_newly_imported_profile() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = isolated_config_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        std::fs::write(
            dir.join("config.toml"),
            r#"
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
known_models = ["deepseek-v4-flash"]
"#,
        )
        .unwrap();
        store_api_key("legacy-deepseek-key").unwrap();

        let loaded = load_startup_config();
        let imported = loaded
            .active_profile()
            .expect("legacy configuration should create an imported profile");

        assert_eq!(imported.name, "Imported DeepSeek");
        assert_eq!(
            load_profile_api_key(&imported.id).unwrap().as_deref(),
            Some("legacy-deepseek-key")
        );
    }

    #[test]
    fn switching_profile_clears_ghost_and_reloads_only_new_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = isolated_config_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        let mut app = AitextApp::new_for_test();
        let first = crate::config::ApiProfile::new("DeepSeek", aitext_ai::ProviderKind::DeepSeek);
        let first_id = first.id.clone();
        app.config.add_profile(first);
        let second = crate::config::ApiProfile::new("OpenAI", aitext_ai::ProviderKind::OpenAi);
        let second_id = second.id.clone();
        app.config.add_profile(second);

        store_profile_api_key(&first_id, "first-profile-key").unwrap();
        store_profile_api_key(&second_id, "second-profile-key").unwrap();

        assert!(app.activate_profile(&first_id));
        assert_eq!(app.api_key.as_deref(), Some("first-profile-key"));
        app.force_ghost(" stale completion");

        assert!(app.activate_profile(&second_id));
        assert!(app.ghost_text().is_none());
        assert_eq!(app.api_key.as_deref(), Some("second-profile-key"));
    }

    #[test]
    fn removing_active_profile_switches_key_and_defers_secret_removal_until_save() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = isolated_config_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        let mut app = AitextApp::new_for_test();
        let first = crate::config::ApiProfile::new("First", aitext_ai::ProviderKind::Custom);
        let first_id = first.id.clone();
        app.config.add_profile(first);
        let second = crate::config::ApiProfile::new("Second", aitext_ai::ProviderKind::Custom);
        let second_id = second.id.clone();
        app.config.add_profile(second);

        store_profile_api_key(&first_id, "first-key").unwrap();
        store_profile_api_key(&second_id, "second-key").unwrap();
        assert!(app.activate_profile(&second_id));
        app.force_ghost("stale ghost");

        assert!(app.remove_profile(&second_id));
        assert_eq!(
            app.config.active_profile_id.as_deref(),
            Some(first_id.as_str())
        );
        assert_eq!(app.api_key.as_deref(), Some("first-key"));
        assert!(app.ghost_text().is_none());
        assert_eq!(
            load_profile_api_key(&second_id).unwrap().as_deref(),
            Some("second-key")
        );

        app.save_settings().unwrap();
        assert_eq!(load_profile_api_key(&second_id).unwrap(), None);
    }

    #[test]
    fn preedit_does_not_touch_document() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.apply_ime(egui::ImeEvent::Preedit("ni".into()));
        assert_eq!(app.workspace.current().unwrap().text(), "");
        assert_eq!(app.ime.preedit, "ni");
        assert!(app.ime.composing);
        app.apply_ime(egui::ImeEvent::Commit("你".into()));
        assert_eq!(app.workspace.current().unwrap().text(), "你");
        assert!(!app.ime.composing);
        assert!(app.workspace.current().unwrap().can_undo());
    }

    #[test]
    fn preedit_suppresses_completion_requests() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.completion.engine.configured = true;
        app.apply_ime(egui::ImeEvent::Preedit("ni".into()));
        app.tick_completion(10_000);
        assert_eq!(
            app.completion.engine.state(),
            aitext_ai::CompletionState::Empty
        );
    }

    #[test]
    fn backspace_clears_stale_ghost() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.workspace.current_mut().unwrap().insert("你们好");
        app.force_ghost("，欢迎来到我的个人网站！");
        app.delete_backward();
        assert_eq!(app.workspace.current().unwrap().text(), "你们");
        assert!(app.completion.engine.suggestion().is_none());
    }

    #[test]
    fn remember_model_from_settings_updates_list() {
        let mut app = AitextApp::new_for_test();
        app.config.add_profile(crate::config::ApiProfile::new(
            "Test API",
            aitext_ai::ProviderKind::Custom,
        ));
        let profile = app.config.active_profile_mut().unwrap();
        profile.remember_model("deepseek-v4-flash");
        profile.remember_model("custom-model");
        assert_eq!(profile.selected_model, "custom-model");
        assert_eq!(profile.known_models[0], "custom-model");
    }

    #[test]
    fn empty_prefix_does_not_keep_ghost() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.force_ghost("，欢迎来到我的个人网站！");
        app.queue_completion();
        assert!(app.completion.engine.suggestion().is_none());
    }
}
use crate::ime::ImeState;
