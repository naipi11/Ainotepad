use aitext_core::{Direction, FindQuery, Match, Workspace};

use aitext_ai::EngineEvent;

use crate::completion::CompletionUiState;
use crate::config::{load_config, AppConfig};

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
    pub status: String,
    pub completion_label: String,
    pub clipboard: String,
    pub pending_api_key: String,
    pub about_open: bool,
    pub shortcuts_open: bool,
    pub completion: CompletionUiState,
    pub api_key: Option<String>,
    pub last_engine_event: Option<EngineEvent>,
    pub ime: ImeState,
}

impl AitextApp {
    pub fn new_for_test() -> Self {
        Self {
            workspace: Workspace::new(),
            config: AppConfig::default(),
            find: FindBarState::default(),
            settings_open: false,
            status: String::new(),
            completion_label: "empty".into(),
            clipboard: String::new(),
            pending_api_key: String::new(),
            about_open: false,
            shortcuts_open: false,
            completion: CompletionUiState::default(),
            api_key: None,
            last_engine_event: None,
            ime: ImeState::default(),
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

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::new_for_test();
        app.config = load_config();
        app.api_key = crate::secrets::load_api_key().ok().flatten();
        app.refresh_completion_config();
        if app.workspace.current_id().is_none() {
            app.workspace.new_untitled();
        }
        app
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
                            self.status = "unsaved changes".into();
                        }
                    }
                    self.workspace.close(id);
                }
            }
            Command::Undo => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.undo();
                }
            }
            Command::Redo => {
                if let Some(doc) = self.workspace.current_mut() {
                    doc.redo();
                }
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
            Command::Open | Command::Save | Command::SaveAs | Command::Exit => {}
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
        app.config.base_url = "https://example.com/v1".into();
        app.config.model = "m".into();
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
        app.config.base_url = "https://api.deepseek.com/v1".into();
        app.config.model = "deepseek-chat".into();
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
        assert_eq!(app.completion.engine.state(), aitext_ai::CompletionState::Empty);
    }
}
use crate::ime::ImeState;
