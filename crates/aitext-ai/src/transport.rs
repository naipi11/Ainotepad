use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::snapshot::CompletionSnapshot;

#[derive(Clone, Debug)]
pub enum CompletionError {
    NotConfigured,
    Timeout,
    AuthFailed,
    Empty,
    RequestFailed(String),
    Cancelled,
}

#[derive(Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub trait Transport: Send + Sync {
    fn complete(
        &self,
        snapshot: CompletionSnapshot,
        cancel: CancelFlag,
    ) -> Result<String, CompletionError>;

    fn complete_streaming(
        &self,
        snapshot: CompletionSnapshot,
        cancel: CancelFlag,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<String, CompletionError> {
        let result = self.complete(snapshot, cancel)?;
        if !result.is_empty() {
            on_chunk(&result);
        }
        Ok(result)
    }
}

pub struct NullTransport;

impl Transport for NullTransport {
    fn complete(
        &self,
        _snapshot: CompletionSnapshot,
        _cancel: CancelFlag,
    ) -> Result<String, CompletionError> {
        Err(CompletionError::NotConfigured)
    }
}

#[cfg(test)]
pub struct FakeTransport {
    pub response: Result<String, CompletionError>,
}

#[cfg(test)]
impl FakeTransport {
    pub fn ok(text: &str) -> Self {
        Self {
            response: Ok(text.into()),
        }
    }

    pub fn fail() -> Self {
        Self {
            response: Err(CompletionError::RequestFailed("nope".into())),
        }
    }
}

#[cfg(test)]
impl Transport for FakeTransport {
    fn complete(
        &self,
        _snapshot: CompletionSnapshot,
        cancel: CancelFlag,
    ) -> Result<String, CompletionError> {
        if cancel.is_cancelled() {
            return Err(CompletionError::Cancelled);
        }
        match &self.response {
            Ok(text) => Ok(text.clone()),
            Err(err) => Err(err.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> CompletionSnapshot {
        CompletionSnapshot {
            document_id: 1,
            prefix: "print(".into(),
            suffix: String::new(),
            file_name: "main.py".into(),
            language: "python".into(),
            generation: 1,
        }
    }

    #[test]
    fn streaming_transport_falls_back_to_one_complete_chunk() {
        let transport = FakeTransport::ok("hello");
        let mut chunks = Vec::new();
        let result = transport.complete_streaming(snapshot(), CancelFlag::new(), &mut |chunk| {
            chunks.push(chunk.to_string());
        });

        assert_eq!(result.unwrap(), "hello");
        assert_eq!(chunks, vec!["hello"]);
    }
}
