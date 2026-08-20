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
