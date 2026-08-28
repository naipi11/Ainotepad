use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use aitext_ai::{
    take_snapshot, CancelFlag, CompletionEngine, CompletionError, CompletionState, EngineEvent,
    NullTransport, OpenAiTransport, ProfileRequestConfig, Transport,
};

use crate::commands::{AitextApp, Command};
use crate::i18n::{completion_state_key, text, UiMessage};
use aitext_core::Motion;

pub(crate) type CompletionInbox = Receiver<CompletionWorkerMessage>;

pub(crate) enum CompletionWorkerMessage {
    Chunk {
        document_id: u64,
        profile_id: String,
        profile_revision: u64,
        completion_generation: u64,
        text: String,
    },
    Finished(CompletionWorkerResult),
}

pub(crate) struct CompletionWorkerResult {
    document_id: u64,
    profile_id: String,
    profile_revision: u64,
    completion_generation: u64,
    result: Result<String, CompletionError>,
}

pub struct CompletionUiState {
    pub engine: CompletionEngine<NullTransport>,
    pub composing: bool,
    pub inflight: Option<u64>,
    pub(crate) inbox: Option<CompletionInbox>,
    pub cancel: CancelFlag,
    pub clock_ms: u64,
}

impl Default for CompletionUiState {
    fn default() -> Self {
        Self {
            engine: CompletionEngine::new(NullTransport),
            composing: false,
            inflight: None,
            inbox: None,
            cancel: CancelFlag::new(),
            clock_ms: 0,
        }
    }
}

impl AitextApp {
    pub fn copy_selection(&mut self) -> String {
        self.clipboard = self
            .workspace
            .current()
            .map(|doc| doc.selected_text())
            .unwrap_or_default();
        self.clipboard.clone()
    }

    pub fn cut_selection(&mut self) -> String {
        let selected = self
            .workspace
            .current()
            .filter(|doc| !doc.is_readonly())
            .map(|doc| doc.selected_text())
            .unwrap_or_default();
        if selected.is_empty() {
            return String::new();
        }
        self.clipboard = selected.clone();
        if let Some(doc) = self.workspace.current_mut() {
            doc.insert("");
        }
        self.invalidate_and_queue();
        selected
    }

    pub fn copy_selection_to_system(&mut self, context: &egui::Context) {
        let text = self.copy_selection();
        if !text.is_empty() {
            context.copy_text(text);
        }
    }

    pub fn cut_selection_to_system(&mut self, context: &egui::Context) {
        let text = self.cut_selection();
        if !text.is_empty() {
            context.copy_text(text);
        }
    }

    pub fn paste_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.clipboard = text.to_string();
        if let Some(doc) = self.workspace.current_mut() {
            if !doc.is_readonly() {
                doc.insert(text);
            }
        }
        self.invalidate_and_queue();
    }

    pub fn paste_from_system(&mut self) {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return;
        };
        let Ok(text) = clipboard.get_text() else {
            return;
        };
        self.paste_text(&text.replace("\r\n", "\n"));
    }

    pub fn delete_backward(&mut self) {
        if let Some(doc) = self.workspace.current_mut() {
            if !doc.is_readonly() {
                doc.delete_backward();
            }
        }
        self.invalidate_and_queue();
    }

    pub fn delete_forward(&mut self) {
        if let Some(doc) = self.workspace.current_mut() {
            if !doc.is_readonly() {
                doc.delete_forward();
            }
        }
        self.invalidate_and_queue();
    }

    pub fn move_caret(&mut self, motion: Motion, extend: bool) {
        if let Some(doc) = self.workspace.current_mut() {
            doc.move_caret(motion, extend);
        }
        self.invalidate_and_queue();
    }

    pub fn note_caret_changed(&mut self) {
        self.invalidate_and_queue();
    }

    pub fn invalidate_and_queue(&mut self) {
        self.cancel_completion();
        self.queue_completion();
    }

    pub fn cancel_completion(&mut self) {
        self.completion.cancel.cancel();
        self.completion.cancel = CancelFlag::new();
        self.completion.inflight = None;
        self.completion.inbox = None;
        self.completion.engine.invalidate();
    }

    pub fn handle_text_input(&mut self, text: &str) {
        let matches_ghost_prefix = self
            .completion
            .engine
            .suggestion()
            .is_some_and(|current| current.text.starts_with(text));
        if matches_ghost_prefix && self.completion.engine.apply_typed_prefix(text).is_some() {
            if let Some(doc) = self.workspace.current_mut() {
                if !doc.is_readonly() {
                    doc.insert(text);
                }
            }
            return;
        }
        if let Some(doc) = self.workspace.current_mut() {
            if !doc.is_readonly() {
                doc.insert(text);
            }
        }
        self.queue_completion();
    }

    pub fn accept_ghost(&mut self) {
        if let Some(text) = self.completion.engine.take_accept() {
            if let Some(doc) = self.workspace.current_mut() {
                doc.insert(&text);
            }
        }
    }

    pub fn reject_ghost(&mut self) {
        self.completion.engine.reject();
    }

    pub fn force_ghost(&mut self, text: &str) {
        self.completion.engine.force_suggestion(text);
    }

    pub fn tick_completion(&mut self, now_ms: u64) {
        self.completion.clock_ms = now_ms;
        self.last_engine_event = Some(self.completion.engine.on_tick(now_ms));
        self.drain_completion(now_ms);
    }

    pub fn queue_completion(&mut self) {
        let Some(doc) = self.workspace.current() else {
            return;
        };
        let snapshot = take_snapshot(doc, 0);
        if snapshot.prefix.trim().is_empty() {
            self.completion.engine.reject();
            return;
        }
        let selection_empty = doc.selection().is_empty();
        let readonly = doc.is_readonly();
        let composing = self.completion.composing;
        self.refresh_completion_config();
        let now_ms = self.completion.clock_ms;
        self.last_engine_event = Some(self.completion.engine.on_change(
            now_ms,
            snapshot,
            selection_empty,
            composing,
            readonly,
            false,
        ));
    }

    pub fn refresh_completion_config(&mut self) {
        self.completion.engine.configured = self.active_request_config().is_some();
        self.completion.engine.enabled = self.config.ghost_enabled;
        self.completion.engine.debounce_ms = self.config.debounce_ms;
    }

    /// Copies the only profile and key that may be used by a background request.
    /// The key stays in memory only long enough to build the worker transport.
    pub fn active_request_config(&self) -> Option<ProfileRequestConfig> {
        let profile = self.config.active_profile()?;
        let api_key = self.api_key.as_deref()?.trim();
        if profile.base_url.trim().is_empty()
            || profile.selected_model.trim().is_empty()
            || api_key.is_empty()
        {
            return None;
        }
        Some(ProfileRequestConfig {
            provider: profile.provider,
            adapter: profile.adapter,
            base_url: profile.base_url.clone(),
            api_key: api_key.to_string(),
            model: profile.selected_model.clone(),
            timeout_ms: profile.timeout_ms,
            allow_http: profile.allow_http,
        })
    }

    pub fn poll_completion(&mut self, now_ms: u64) {
        self.completion.clock_ms = now_ms;
        self.refresh_completion_config();
        self.drain_completion(now_ms);
        let event = self.completion.engine.on_tick(now_ms);
        self.last_engine_event = Some(event);
        if let Some(EngineEvent::StartRequest { snapshot }) = &self.last_engine_event {
            self.start_completion_request(snapshot.clone());
        }
    }

    fn drain_completion(&mut self, now_ms: u64) {
        let Some(inbox) = self.completion.inbox.take() else {
            return;
        };
        loop {
            match inbox.try_recv() {
                Ok(CompletionWorkerMessage::Chunk {
                    document_id,
                    profile_id,
                    profile_revision,
                    completion_generation,
                    text,
                }) => {
                    let still_current = self.config.active_profile_id.as_deref()
                        == Some(profile_id.as_str())
                        && self
                            .workspace
                            .current()
                            .is_some_and(|doc| doc.id() == document_id)
                        && profile_revision == self.profile_revision
                        && completion_generation == self.completion.engine.generation();
                    if still_current {
                        self.last_engine_event = Some(self.completion.engine.on_stream_chunk_at(
                            completion_generation,
                            &text,
                            now_ms,
                        ));
                    }
                }
                Ok(CompletionWorkerMessage::Finished(worker_result)) => {
                    let still_current = self.config.active_profile_id.as_deref()
                        == Some(worker_result.profile_id.as_str())
                        && self
                            .workspace
                            .current()
                            .is_some_and(|doc| doc.id() == worker_result.document_id)
                        && worker_result.profile_revision == self.profile_revision
                        && worker_result.completion_generation
                            == self.completion.engine.generation();
                    if !still_current {
                        continue;
                    }

                    self.last_engine_event = Some(self.completion.engine.on_result_at(
                        worker_result.completion_generation,
                        worker_result.result,
                        now_ms,
                    ));
                    self.completion.inflight = None;
                    if let Some(detail) = self.completion.engine.last_error() {
                        self.status = Some(if detail.chars().count() > 48 {
                            UiMessage::NoSuggestion
                        } else {
                            UiMessage::CompletionDetail(detail.to_string())
                        });
                    } else if matches!(
                        self.completion.engine.state(),
                        CompletionState::Suggested
                            | CompletionState::Empty
                            | CompletionState::NoSuggestion
                    ) && self
                        .status
                        .as_ref()
                        .is_some_and(UiMessage::is_completion_feedback)
                    {
                        self.status = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.completion.inflight = None;
                    break;
                }
            }
        }
        if self.completion.inflight.is_some() {
            self.completion.inbox = Some(inbox);
        }
    }

    fn start_completion_request(&mut self, snapshot: aitext_ai::CompletionSnapshot) {
        let Some(profile_id) = self
            .config
            .active_profile()
            .map(|profile| profile.id.clone())
        else {
            self.last_engine_event = Some(
                self.completion
                    .engine
                    .on_result(snapshot.generation, Err(CompletionError::NotConfigured)),
            );
            return;
        };
        let Some(request_config) = self.active_request_config() else {
            self.last_engine_event = Some(
                self.completion
                    .engine
                    .on_result(snapshot.generation, Err(CompletionError::NotConfigured)),
            );
            return;
        };
        self.completion.cancel.cancel();
        self.completion.cancel = CancelFlag::new();
        let cancel = self.completion.cancel.clone();
        let (tx, rx) = mpsc::channel();
        self.completion.inbox = Some(rx);
        self.completion.inflight = Some(snapshot.generation);
        let transport = OpenAiTransport {
            config: request_config,
        };
        let profile_revision = self.profile_revision;
        let completion_generation = snapshot.generation;
        let document_id = snapshot.document_id;
        thread::spawn(move || {
            let tx_for_chunk = tx.clone();
            let profile_id_for_chunk = profile_id.clone();
            let mut emit_chunk = |text: &str| {
                let _ = tx_for_chunk.send(CompletionWorkerMessage::Chunk {
                    document_id,
                    profile_id: profile_id_for_chunk.clone(),
                    profile_revision,
                    completion_generation,
                    text: text.to_string(),
                });
            };
            let result = transport.complete_streaming(snapshot, cancel, &mut emit_chunk);
            let _ = tx.send(CompletionWorkerMessage::Finished(CompletionWorkerResult {
                document_id,
                profile_id,
                profile_revision,
                completion_generation,
                result,
            }));
        });
    }
}

impl AitextApp {
    pub fn ghost_text(&self) -> Option<&str> {
        self.completion.engine.suggestion().map(|s| s.text.as_str())
    }

    pub fn completion_metrics(&self) -> aitext_ai::CompletionMetrics {
        self.completion.engine.metrics()
    }

    pub fn completion_label_now(&self) -> &'static str {
        text(
            self.locale(),
            completion_state_key(self.completion.engine.state()),
        )
    }
}

pub fn apply_completion_command(app: &mut AitextApp, command: Command) {
    match command {
        Command::AcceptGhost => app.accept_ghost(),
        Command::RejectGhost => app.reject_ghost(),
        _ => {}
    }
}

#[allow(dead_code)]
fn state_from_engine(state: CompletionState) -> &'static str {
    state.label()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::UiLanguage;

    #[test]
    fn completion_label_follows_the_ui_language() {
        let mut app = AitextApp::new_for_test();
        assert_eq!(app.completion_label_now(), "empty");

        app.set_ui_language(UiLanguage::ZhCn);
        assert_eq!(app.completion_label_now(), "空闲");
    }

    #[test]
    fn app_exposes_in_memory_completion_metrics_for_diagnostics() {
        let app = AitextApp::new_for_test();
        assert_eq!(
            app.completion_metrics(),
            aitext_ai::CompletionMetrics::default()
        );
    }

    #[test]
    fn worker_chunk_updates_ghost_before_final_result_arrives() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let profile = crate::config::ApiProfile::new("Test", aitext_ai::ProviderKind::Custom);
        let profile_id = profile.id.clone();
        app.config.add_profile(profile);

        app.completion.engine.configured = true;
        let snapshot = aitext_ai::CompletionSnapshot {
            document_id: 1,
            prefix: "print(\"hel".into(),
            suffix: String::new(),
            file_name: "main.py".into(),
            language: "python".into(),
            generation: 0,
        };
        app.completion
            .engine
            .on_change(0, snapshot, true, false, false, false);
        let generation = match app.completion.engine.on_tick(60) {
            aitext_ai::EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected completion request"),
        };
        app.completion.inflight = Some(generation);

        let (tx, rx) = mpsc::channel();
        app.completion.inbox = Some(rx);
        tx.send(CompletionWorkerMessage::Chunk {
            document_id: 1,
            profile_id,
            profile_revision: app.profile_revision,
            completion_generation: generation,
            text: "lo".into(),
        })
        .unwrap();

        app.drain_completion(100);

        assert_eq!(app.ghost_text(), Some("lo\")"));
    }

    #[test]
    fn worker_chunk_for_another_document_is_ignored() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let profile = crate::config::ApiProfile::new("Test", aitext_ai::ProviderKind::Custom);
        let profile_id = profile.id.clone();
        app.config.add_profile(profile);
        app.completion.engine.configured = true;
        let snapshot = aitext_ai::CompletionSnapshot {
            document_id: 1,
            prefix: "fn main() {".into(),
            suffix: String::new(),
            file_name: "main.rs".into(),
            language: "rust".into(),
            generation: 0,
        };
        app.completion
            .engine
            .on_change(0, snapshot, true, false, false, false);
        let generation = match app.completion.engine.on_tick(60) {
            aitext_ai::EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected completion request"),
        };
        app.completion.inflight = Some(generation);
        let (tx, rx) = mpsc::channel();
        app.completion.inbox = Some(rx);
        tx.send(CompletionWorkerMessage::Chunk {
            document_id: 999,
            profile_id,
            profile_revision: app.profile_revision,
            completion_generation: generation,
            text: "wrong document".into(),
        })
        .unwrap();

        app.drain_completion(100);

        assert!(app.ghost_text().is_none());
    }

    #[test]
    fn clearing_document_while_requesting_rejects_a_late_result() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        app.workspace.current_mut().unwrap().insert("x");
        let mut profile = crate::config::ApiProfile::new("Test", aitext_ai::ProviderKind::Custom);
        profile.base_url = "https://example.com/v1".into();
        profile.remember_model("test-model");
        app.config.add_profile(profile);
        app.api_key = Some("test-key".into());
        app.completion.engine.configured = true;

        let snapshot = aitext_ai::CompletionSnapshot {
            document_id: 1,
            prefix: "fn main() {".into(),
            suffix: String::new(),
            file_name: "main.rs".into(),
            language: "rust".into(),
            generation: 0,
        };
        app.completion
            .engine
            .on_change(0, snapshot, true, false, false, false);
        let generation = match app.completion.engine.on_tick(60) {
            aitext_ai::EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected completion request"),
        };
        app.completion.inflight = Some(generation);

        app.delete_backward();
        app.completion
            .engine
            .on_result(generation, Ok(" late result".into()));

        assert_eq!(app.workspace.current().unwrap().text(), "");
        assert!(app.ghost_text().is_none());
        assert_ne!(app.completion.engine.generation(), generation);
    }

    #[test]
    fn python_language_change_keeps_completion_requests_active() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let mut profile =
            crate::config::ApiProfile::new("DeepSeek", aitext_ai::ProviderKind::DeepSeek);
        profile.base_url = "https://api.deepseek.com/v1".into();
        profile.remember_model("deepseek-v4-flash");
        app.config.add_profile(profile);
        app.api_key = Some("sk-test".into());
        app.handle_text_input("print(\"Hello\")");
        app.set_document_language(aitext_core::LanguageId::Python);
        app.handle_text_input("\nprint(\"My name is ");
        app.poll_completion(10_000);

        match app.last_engine_event.as_ref() {
            Some(aitext_ai::EngineEvent::StartRequest { snapshot }) => {
                assert_eq!(snapshot.language, "python");
            }
            _ => panic!("expected a Python completion request"),
        }
    }

    #[test]
    fn stale_worker_result_cannot_apply_after_profile_revision_changes() {
        let mut app = AitextApp::new_for_test();
        app.dispatch(Command::NewTab);
        let mut profile = crate::config::ApiProfile::new("OpenAI", aitext_ai::ProviderKind::OpenAi);
        profile.base_url = "https://api.openai.com/v1".into();
        profile.remember_model("gpt-test");
        app.config.add_profile(profile);
        app.api_key = Some("profile-key".into());
        app.refresh_completion_config();

        let profile_id = app.config.active_profile().unwrap().id.clone();
        let old_revision = app.profile_revision;
        let old_generation = app.completion.engine.generation();
        app.profile_changed();
        app.status = Some(UiMessage::SettingsSaved);

        let (tx, rx) = mpsc::channel();
        app.completion.inbox = Some(rx);
        tx.send(CompletionWorkerMessage::Finished(CompletionWorkerResult {
            document_id: 1,
            profile_id,
            profile_revision: old_revision,
            completion_generation: old_generation,
            result: Ok("stale completion".into()),
        }))
        .unwrap();

        app.drain_completion(100);

        assert!(app.ghost_text().is_none());
        assert_eq!(app.status, Some(UiMessage::SettingsSaved));
    }
}
