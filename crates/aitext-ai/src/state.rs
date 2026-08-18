#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionState {
    Empty,
    Requesting,
    Suggested,
    NotConfigured,
    Timeout,
    AuthFailed,
    NoSuggestion,
    RequestFailed,
}

impl CompletionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Requesting => "requesting",
            Self::Suggested => "suggested",
            Self::NotConfigured => "not configured",
            Self::Timeout => "timeout",
            Self::AuthFailed => "auth failed",
            Self::NoSuggestion => "no suggestion",
            Self::RequestFailed => "request failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhostSuggestion {
    pub text: String,
    pub generation: u64,
}
