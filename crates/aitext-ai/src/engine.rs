use crate::shape::shape_suggestion;
use crate::snapshot::CompletionSnapshot;
use crate::state::{CompletionState, GhostSuggestion};
use crate::transport::{CompletionError, Transport};

pub const DEFAULT_DEBOUNCE_MS: u64 = 250;
pub const FAILURES_BEFORE_BACKOFF: u32 = 3;
pub const BACKOFF_MS: u64 = 5000;

pub enum EngineEvent {
    None,
    StartRequest { snapshot: CompletionSnapshot },
    CancelRequest { generation: u64 },
    StateChanged,
}

pub struct CompletionEngine<T: Transport> {
    pub transport: T,
    pub debounce_ms: u64,
    pub configured: bool,
    pub enabled: bool,
    generation: u64,
    pending: Option<(u64, CompletionSnapshot)>,
    suggestion: Option<GhostSuggestion>,
    state: CompletionState,
    failures: u32,
    backoff_until: u64,
    inflight: Option<u64>,
    last_error: Option<String>,
    inflight_prefix: String,
}

impl<T: Transport> CompletionEngine<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            configured: false,
            enabled: true,
            generation: 0,
            pending: None,
            suggestion: None,
            state: CompletionState::Empty,
            failures: 0,
            backoff_until: 0,
            inflight: None,
            last_error: None,
            inflight_prefix: String::new(),
        }
    }

    pub fn suggestion(&self) -> Option<&GhostSuggestion> {
        self.suggestion.as_ref()
    }

    pub fn state(&self) -> CompletionState {
        self.state
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn reject(&mut self) {
        self.suggestion = None;
        if self.state == CompletionState::Suggested {
            self.state = CompletionState::Empty;
        }
    }

    pub fn take_accept(&mut self) -> Option<String> {
        let text = self.suggestion.take().map(|s| s.text);
        if text.is_some() {
            self.state = CompletionState::Empty;
        }
        text
    }

    pub fn force_suggestion(&mut self, text: &str) {
        self.suggestion = Some(GhostSuggestion {
            text: text.to_string(),
            generation: self.generation,
        });
        self.state = CompletionState::Suggested;
    }

    pub fn apply_typed_prefix(&mut self, typed: &str) -> Option<GhostSuggestion> {
        let Some(current) = self.suggestion.as_mut() else {
            return None;
        };
        if current.text.starts_with(typed) {
            current.text = current.text[typed.len()..].to_string();
            if current.text.is_empty() {
                self.suggestion = None;
                self.state = CompletionState::Empty;
                return None;
            }
            return self.suggestion.clone();
        }
        self.suggestion = None;
        self.state = CompletionState::Empty;
        None
    }

    pub fn on_change(
        &mut self,
        now_ms: u64,
        mut snapshot: CompletionSnapshot,
        selection_empty: bool,
        composing: bool,
        readonly: bool,
        too_large: bool,
    ) -> EngineEvent {
        self.generation += 1;
        snapshot.generation = self.generation;
        self.suggestion = None;
        if !self.configured {
            self.state = CompletionState::NotConfigured;
            self.pending = None;
            return EngineEvent::StateChanged;
        }
        if !self.enabled || composing || !selection_empty || readonly || too_large || now_ms < self.backoff_until
        {
            self.pending = None;
            if now_ms < self.backoff_until {
                self.state = CompletionState::RequestFailed;
            } else if self.state == CompletionState::Suggested {
                self.state = CompletionState::Empty;
            }
            return EngineEvent::None;
        }
        self.pending = Some((now_ms, snapshot));
        EngineEvent::None
    }

    pub fn on_tick(&mut self, now_ms: u64) -> EngineEvent {
        let Some((started, snapshot)) = self.pending.clone() else {
            return EngineEvent::None;
        };
        if now_ms.saturating_sub(started) < self.debounce_ms {
            return EngineEvent::None;
        }
        if now_ms < self.backoff_until {
            return EngineEvent::None;
        }
        self.pending = None;
        self.inflight = Some(snapshot.generation);
        self.inflight_prefix = snapshot.prefix.clone();
        self.state = CompletionState::Requesting;
        EngineEvent::StartRequest { snapshot }
    }

    pub fn on_result(&mut self, generation: u64, result: Result<String, CompletionError>) -> EngineEvent {
        self.on_result_at(generation, result, 0)
    }

    pub fn on_result_at(
        &mut self,
        generation: u64,
        result: Result<String, CompletionError>,
        now_ms: u64,
    ) -> EngineEvent {
        if generation != self.generation {
            return EngineEvent::None;
        }
        self.inflight = None;
        match result {
            Ok(raw) => {
                let prefix_tail: String = self
                    .inflight_prefix
                    .chars()
                    .rev()
                    .take(80)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                if let Some(text) = shape_suggestion(&raw, &prefix_tail) {
                    self.suggestion = Some(GhostSuggestion { text, generation });
                    self.state = CompletionState::Suggested;
                    self.failures = 0;
                    self.backoff_until = 0;
                    self.last_error = None;
                } else {
                    self.suggestion = None;
                    self.state = CompletionState::NoSuggestion;
                    self.last_error = Some("empty completion".into());
                }
            }
            Err(CompletionError::Cancelled) => {}
            Err(CompletionError::Empty) => {
                self.state = CompletionState::NoSuggestion;
                self.last_error = Some("empty completion".into());
            }
            Err(CompletionError::NotConfigured) => {
                self.state = CompletionState::NotConfigured;
                self.last_error = Some("missing url, model, or api key".into());
            }
            Err(CompletionError::Timeout) => {
                self.note_failure(now_ms);
                self.state = CompletionState::Timeout;
                self.last_error = Some("timeout".into());
            }
            Err(CompletionError::AuthFailed) => {
                self.note_failure(now_ms);
                self.state = CompletionState::AuthFailed;
                self.last_error = Some("auth failed".into());
            }
            Err(CompletionError::RequestFailed(msg)) => {
                self.note_failure(now_ms);
                self.state = CompletionState::RequestFailed;
                self.last_error = Some(msg);
            }
        }
        EngineEvent::StateChanged
    }

    fn note_failure(&mut self, now_ms: u64) {
        self.failures += 1;
        if self.failures >= FAILURES_BEFORE_BACKOFF {
            self.backoff_until = now_ms.saturating_add(BACKOFF_MS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CompletionSnapshot;
    use crate::transport::FakeTransport;

    fn sample_snapshot() -> CompletionSnapshot {
        CompletionSnapshot {
            document_id: 1,
            prefix: "fn main() {".into(),
            suffix: String::new(),
            file_name: "main.rs".into(),
            language: "rust".into(),
            generation: 0,
        }
    }

    #[test]
    fn debounce_then_start_request() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("xyz"));
        engine.configured = true;
        let ev = engine.on_change(0, sample_snapshot(), true, false, false, false);
        assert!(matches!(ev, EngineEvent::None));
        let ev = engine.on_tick(249);
        assert!(matches!(ev, EngineEvent::None));
        let ev = engine.on_tick(250);
        match ev {
            EngineEvent::StartRequest { snapshot } => assert_eq!(snapshot.generation, 1),
            _ => panic!("expected start"),
        }
    }

    #[test]
    fn stale_generation_is_ignored() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("xyz"));
        engine.configured = true;
        engine.on_change(0, sample_snapshot(), true, false, false, false);
        engine.on_tick(250);
        engine.on_change(300, sample_snapshot(), true, false, false, false);
        engine.on_result(1, Ok("old".into()));
        assert!(engine.suggestion().is_none());
    }

    #[test]
    fn typed_prefix_trims_locally() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("xyz"));
        engine.force_suggestion("hello");
        assert_eq!(engine.apply_typed_prefix("he").unwrap().text, "llo");
    }

    #[test]
    fn three_failures_enter_backoff() {
        let mut engine = CompletionEngine::new(FakeTransport::fail());
        engine.configured = true;
        for i in 1..=3 {
            engine.on_change(i as u64 * 1000, sample_snapshot(), true, false, false, false);
            let ev = engine.on_tick(i as u64 * 1000 + 250);
            let gen = match ev {
                EngineEvent::StartRequest { snapshot } => snapshot.generation,
                _ => panic!("expected start"),
            };
            engine.on_result_at(gen, Err(CompletionError::RequestFailed("nope".into())), i as u64 * 1000 + 250);
        }
        engine.on_change(4000, sample_snapshot(), true, false, false, false);
        let ev = engine.on_tick(4250);
        assert!(matches!(ev, EngineEvent::None));
        assert_eq!(engine.state(), CompletionState::RequestFailed);
    }
}
