use crate::shape::shape_suggestion_for_context;
use crate::snapshot::CompletionSnapshot;
use crate::state::{CompletionState, GhostSuggestion};
use crate::transport::{CompletionError, Transport};

pub const DEFAULT_DEBOUNCE_MS: u64 = 60;
pub const FAILURES_BEFORE_BACKOFF: u32 = 3;
pub const BACKOFF_MS: u64 = 5000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletionMetrics {
    pub requests_started: u64,
    pub suggestions_shown: u64,
    pub suggestions_accepted: u64,
    pub cancellations: u64,
    pub failures: u64,
    pub last_time_to_first_chunk_ms: Option<u64>,
    pub last_completion_latency_ms: Option<u64>,
}

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
    inflight_suffix: String,
    inflight_language: String,
    inflight_text: String,
    metrics: CompletionMetrics,
    request_started_at_ms: Option<u64>,
    first_chunk_seen: bool,
    suggestion_reported: bool,
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
            inflight_suffix: String::new(),
            inflight_language: String::new(),
            inflight_text: String::new(),
            metrics: CompletionMetrics::default(),
            request_started_at_ms: None,
            first_chunk_seen: false,
            suggestion_reported: false,
        }
    }

    pub fn suggestion(&self) -> Option<&GhostSuggestion> {
        self.suggestion.as_ref()
    }

    pub fn state(&self) -> CompletionState {
        self.state
    }

    /// Identifies the currently valid document/configuration snapshot.
    /// Background workers must echo this value before their result can apply.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn metrics(&self) -> CompletionMetrics {
        self.metrics.clone()
    }

    pub fn reject(&mut self) {
        self.suggestion = None;
        if self.state == CompletionState::Suggested {
            self.state = CompletionState::Empty;
        }
    }

    pub fn invalidate(&mut self) {
        if self.inflight.is_some() {
            self.metrics.cancellations += 1;
        }
        self.generation += 1;
        self.pending = None;
        self.inflight = None;
        self.inflight_prefix.clear();
        self.inflight_suffix.clear();
        self.inflight_language.clear();
        self.inflight_text.clear();
        self.request_started_at_ms = None;
        self.first_chunk_seen = false;
        self.suggestion_reported = false;
        self.suggestion = None;
        self.last_error = None;
        self.failures = 0;
        self.backoff_until = 0;
        self.state = if self.configured {
            CompletionState::Empty
        } else {
            CompletionState::NotConfigured
        };
    }

    pub fn take_accept(&mut self) -> Option<String> {
        let text = self.suggestion.take().map(|s| s.text);
        if text.is_some() {
            self.metrics.suggestions_accepted += 1;
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
        let current = self.suggestion.as_mut()?;
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
        if self.inflight.is_some() {
            self.metrics.cancellations += 1;
        }
        self.generation += 1;
        snapshot.generation = self.generation;
        self.suggestion = None;
        self.inflight_text.clear();
        if !self.configured {
            self.state = CompletionState::NotConfigured;
            self.pending = None;
            return EngineEvent::StateChanged;
        }
        if !self.enabled
            || composing
            || !selection_empty
            || readonly
            || too_large
            || now_ms < self.backoff_until
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
        self.inflight_suffix = snapshot.suffix.clone();
        self.inflight_language = snapshot.language.clone();
        self.inflight_text.clear();
        self.metrics.requests_started += 1;
        self.request_started_at_ms = Some(now_ms);
        self.first_chunk_seen = false;
        self.suggestion_reported = false;
        self.state = CompletionState::Requesting;
        EngineEvent::StartRequest { snapshot }
    }

    pub fn on_result(
        &mut self,
        generation: u64,
        result: Result<String, CompletionError>,
    ) -> EngineEvent {
        self.on_result_at(generation, result, 0)
    }

    pub fn on_stream_chunk(&mut self, generation: u64, chunk: &str) -> EngineEvent {
        self.on_stream_chunk_at(generation, chunk, 0)
    }

    pub fn on_stream_chunk_at(&mut self, generation: u64, chunk: &str, now_ms: u64) -> EngineEvent {
        if generation != self.generation || self.inflight != Some(generation) {
            return EngineEvent::None;
        }
        if !self.first_chunk_seen && !chunk.is_empty() {
            self.metrics.last_time_to_first_chunk_ms = self
                .request_started_at_ms
                .map(|started| now_ms.saturating_sub(started));
            self.first_chunk_seen = true;
        }
        self.inflight_text.push_str(chunk);
        if let Some(text) = shape_suggestion_for_context(
            &self.inflight_text,
            &self.inflight_prefix,
            &self.inflight_suffix,
            &self.inflight_language,
        ) {
            self.suggestion = Some(GhostSuggestion { text, generation });
            self.state = CompletionState::Suggested;
            if !self.suggestion_reported {
                self.metrics.suggestions_shown += 1;
                self.suggestion_reported = true;
            }
            self.last_error = None;
        } else {
            self.suggestion = None;
            self.state = CompletionState::NoSuggestion;
            self.last_error = None;
        }
        EngineEvent::StateChanged
    }

    pub fn on_result_at(
        &mut self,
        generation: u64,
        result: Result<String, CompletionError>,
        now_ms: u64,
    ) -> EngineEvent {
        if generation != self.generation || self.inflight != Some(generation) {
            return EngineEvent::None;
        }
        if let Some(started) = self.request_started_at_ms {
            self.metrics.last_completion_latency_ms = Some(now_ms.saturating_sub(started));
        }
        match &result {
            Err(CompletionError::Cancelled) => self.metrics.cancellations += 1,
            Err(_) => self.metrics.failures += 1,
            Ok(_) => {}
        }
        self.inflight = None;
        match result {
            Ok(raw) => {
                if let Some(text) = shape_suggestion_for_context(
                    &raw,
                    &self.inflight_prefix,
                    &self.inflight_suffix,
                    &self.inflight_language,
                ) {
                    self.suggestion = Some(GhostSuggestion { text, generation });
                    self.state = CompletionState::Suggested;
                    if !self.suggestion_reported {
                        self.metrics.suggestions_shown += 1;
                        self.suggestion_reported = true;
                    }
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
        let ev = engine.on_tick(59);
        assert!(matches!(ev, EngineEvent::None));
        let ev = engine.on_tick(60);
        match ev {
            EngineEvent::StartRequest { snapshot } => assert_eq!(snapshot.generation, 1),
            _ => panic!("expected start"),
        }
    }

    #[test]
    fn engine_keeps_python_call_closer_in_the_visible_suggestion() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        let mut snapshot = sample_snapshot();
        snapshot.prefix = "print(".into();
        snapshot.language = "python".into();
        engine.on_change(0, snapshot, true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };
        engine.on_result(generation, Ok("\"Hello, World!\n".into()));
        assert_eq!(
            engine
                .suggestion()
                .map(|suggestion| suggestion.text.as_str()),
            Some("\"Hello, World!\")")
        );
    }

    #[test]
    fn streaming_chunks_update_one_visible_suggestion_and_ignore_stale_chunks() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        let mut snapshot = sample_snapshot();
        snapshot.prefix = "print(\"hel".into();
        engine.on_change(0, snapshot, true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_stream_chunk(generation, "lo");
        assert_eq!(
            engine.suggestion().map(|item| item.text.as_str()),
            Some("lo\")")
        );
        engine.on_stream_chunk(generation, " world");
        assert_eq!(
            engine.suggestion().map(|item| item.text.as_str()),
            Some("lo world\")")
        );

        engine.on_stream_chunk(generation - 1, " stale");
        assert_eq!(
            engine.suggestion().map(|item| item.text.as_str()),
            Some("lo world\")")
        );
    }

    #[test]
    fn metrics_record_stream_latency_and_acceptance() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        let mut snapshot = sample_snapshot();
        snapshot.prefix = "fn main() {".into();
        engine.on_change(0, snapshot, true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_stream_chunk_at(generation, " println!(\"hi\");", 85);
        engine.on_result_at(generation, Ok(" println!(\"hi\");".into()), 120);
        let accepted = engine.take_accept();
        assert!(accepted.is_some());

        let metrics = engine.metrics();
        assert_eq!(metrics.requests_started, 1);
        assert_eq!(metrics.suggestions_shown, 1);
        assert_eq!(metrics.suggestions_accepted, 1);
        assert_eq!(metrics.last_time_to_first_chunk_ms, Some(25));
        assert_eq!(metrics.last_completion_latency_ms, Some(60));
    }

    #[test]
    fn metrics_separate_cancellation_from_provider_failure() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        engine.on_change(0, sample_snapshot(), true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_result_at(generation, Err(CompletionError::Cancelled), 70);

        let metrics = engine.metrics();
        assert_eq!(metrics.cancellations, 1);
        assert_eq!(metrics.failures, 0);
    }

    #[test]
    fn duplicate_final_result_cannot_restore_an_accepted_suggestion() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        engine.on_change(0, sample_snapshot(), true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        let result = Ok(" println!(\"ok\");".into());
        engine.on_result_at(generation, result, 70);
        assert!(engine.take_accept().is_some());
        assert!(engine.suggestion().is_none());

        engine.on_result_at(generation, Ok(" duplicate".into()), 80);

        assert!(engine.suggestion().is_none());
    }

    #[test]
    fn engine_removes_existing_suffix_from_final_completion() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        let mut snapshot = sample_snapshot();
        snapshot.prefix = "Hello".into();
        snapshot.suffix = " world".into();
        snapshot.language = "plain".into();
        engine.on_change(0, snapshot, true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_result_at(generation, Ok(" world is bright".into()), 70);

        assert_eq!(
            engine
                .suggestion()
                .map(|suggestion| suggestion.text.as_str()),
            Some(" is bright")
        );
    }

    #[test]
    fn rejected_provider_text_is_not_copied_into_diagnostics() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        engine.on_change(0, sample_snapshot(), true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_result_at(
            generation,
            Ok("complete this code: PRIVATE_DOCUMENT_FRAGMENT".into()),
            70,
        );

        assert_eq!(engine.last_error(), Some("empty completion"));
    }

    #[test]
    fn invalid_streaming_tail_clears_an_older_preview() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("unused"));
        engine.configured = true;
        let mut snapshot = sample_snapshot();
        snapshot.prefix = "hello".into();
        engine.on_change(0, snapshot, true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected a completion request"),
        };

        engine.on_stream_chunk(generation, " world");
        assert!(engine.suggestion().is_some());
        engine.on_stream_chunk(generation, " We need to explain this");

        assert!(engine.suggestion().is_none());
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
    fn engine_invalidate_makes_old_snapshot_stale() {
        let mut engine = CompletionEngine::new(FakeTransport::ok("xyz"));
        engine.configured = true;
        engine.on_change(0, sample_snapshot(), true, false, false, false);
        let generation = match engine.on_tick(60) {
            EngineEvent::StartRequest { snapshot } => snapshot.generation,
            _ => panic!("expected request"),
        };

        engine.invalidate();
        engine.on_result(generation, Ok("old completion".into()));

        assert!(engine.suggestion().is_none());
        assert_eq!(engine.state(), CompletionState::Empty);
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
            engine.on_change(
                i as u64 * 1000,
                sample_snapshot(),
                true,
                false,
                false,
                false,
            );
            let ev = engine.on_tick(i as u64 * 1000 + 250);
            let gen = match ev {
                EngineEvent::StartRequest { snapshot } => snapshot.generation,
                _ => panic!("expected start"),
            };
            engine.on_result_at(
                gen,
                Err(CompletionError::RequestFailed("nope".into())),
                i as u64 * 1000 + 250,
            );
        }
        engine.on_change(4000, sample_snapshot(), true, false, false, false);
        let ev = engine.on_tick(4250);
        assert!(matches!(ev, EngineEvent::None));
        assert_eq!(engine.state(), CompletionState::RequestFailed);
    }
}
