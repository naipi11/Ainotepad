use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;


use aitext_ai::{
    take_snapshot, CancelFlag, CompletionEngine, CompletionError, CompletionState, EngineEvent,
    NullTransport, OpenAiConfig, OpenAiTransport, Transport,
};

use crate::commands::{AitextApp, Command};

type CompletionInbox = Receiver<(u64, Result<String, CompletionError>)>;

pub struct CompletionUiState {
    pub engine: CompletionEngine<NullTransport>,
    pub composing: bool,
    pub inflight: Option<u64>,
    pub inbox: Option<CompletionInbox>,
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
        self.completion.clock_ms = now_ms;
        self.last_engine_event = Some(self.completion.engine.on_tick(now_ms));
        self.drain_completion(now_ms);
    }

    pub fn queue_completion(&mut self) {
        let Some(doc) = self.workspace.current() else {
            return;
        };
        let snapshot = take_snapshot(doc, 0);
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
        let configured = !self.config.base_url.trim().is_empty()
            && !self.config.model.trim().is_empty()
            && self.api_key.as_ref().map(|k| !k.trim().is_empty()).unwrap_or(false);
        self.completion.engine.configured = configured;
        self.completion.engine.enabled = self.config.ghost_enabled;
        self.completion.engine.debounce_ms = self.config.debounce_ms;
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
                Ok((generation, result)) => {
                    self.last_engine_event =
                        Some(self.completion.engine.on_result_at(generation, result, now_ms));
                    self.completion.inflight = None;
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
        let Some(api_key) = self.api_key.clone().filter(|k| !k.trim().is_empty()) else {
            self.last_engine_event = Some(self.completion.engine.on_result(
                snapshot.generation,
                Err(CompletionError::NotConfigured),
            ));
            return;
        };
        self.completion.cancel.cancel();
        self.completion.cancel = CancelFlag::new();
        let cancel = self.completion.cancel.clone();
        let (tx, rx) = mpsc::channel();
        self.completion.inbox = Some(rx);
        self.completion.inflight = Some(snapshot.generation);
        let transport = OpenAiTransport {
            config: OpenAiConfig {
                base_url: self.config.base_url.clone(),
                api_key,
                model: self.config.model.clone(),
                timeout_ms: self.config.timeout_ms,
                allow_http: self.config.allow_http,
            },
        };
        thread::spawn(move || {
            let result = transport.complete(snapshot.clone(), cancel);
            let _ = tx.send((snapshot.generation, result));
        });
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
