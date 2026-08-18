use aitext_ai::{
    take_snapshot, CompletionEngine, CompletionState, NullTransport,
};

use crate::commands::{AitextApp, Command};

pub struct CompletionUiState {
    pub engine: CompletionEngine<NullTransport>,
    pub composing: bool,
    pub inflight: Option<u64>,
}

impl Default for CompletionUiState {
    fn default() -> Self {
        Self {
            engine: CompletionEngine::new(NullTransport),
            composing: false,
            inflight: None,
        }
    }
}

impl AitextApp {
    pub fn handle_text_input(&mut self, text: &str) {
        if let Some(current) = self.completion.engine.suggestion() {
            if current.text.starts_with(text) {
                if self.completion.engine.apply_typed_prefix(text).is_some() {
                    if let Some(doc) = self.workspace.current_mut() {
                        if !doc.is_readonly() {
                            doc.insert(text);
                        }
                    }
                    return;
                }
            }
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
        self.last_engine_event = Some(self.completion.engine.on_tick(now_ms));
    }

    pub fn queue_completion(&mut self) {
        let Some(doc) = self.workspace.current() else {
            return;
        };
        let configured = !self.config.base_url.is_empty()
            && !self.config.model.is_empty()
            && self.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);
        self.completion.engine.configured = configured;
        self.completion.engine.enabled = self.config.ghost_enabled;
        self.completion.engine.debounce_ms = self.config.debounce_ms;
        let snapshot = take_snapshot(doc, 0);
        self.last_engine_event = Some(self.completion.engine.on_change(
            0,
            snapshot,
            doc.selection().is_empty(),
            self.completion.composing,
            doc.is_readonly(),
            false,
        ));
    }
}

impl AitextApp {
    pub fn ghost_text(&self) -> Option<&str> {
        self.completion.engine.suggestion().map(|s| s.text.as_str())
    }

    pub fn completion_label_now(&self) -> &'static str {
        self.completion.engine.state().label()
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
