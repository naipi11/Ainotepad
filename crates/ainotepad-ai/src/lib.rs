pub mod engine;
pub mod openai;
pub mod shape;
pub mod snapshot;
pub mod state;
pub mod transport;

use serde::{Deserialize, Serialize};

/// The provider family selected for one saved API profile.
///
/// The value is explicit so a URL cannot make one provider reuse another
/// provider's authentication or fixed transport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    DeepSeek,
    #[doc(hidden)]
    DeepSeekFim,
    OpenAi,
    Xai,
    Anthropic,
    #[serde(alias = "open_ai_compatible")]
    #[default]
    Custom,
}

/// The selectable OpenAI-shaped request contract used by OpenAI and Custom.
/// Provider-native DeepSeek, xAI, and Anthropic transports ignore this field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Fim,
    #[default]
    ChatCompletions,
    Responses,
}

impl ProviderKind {
    pub fn default_adapter(self) -> AdapterKind {
        match self {
            Self::DeepSeek | Self::DeepSeekFim => AdapterKind::Fim,
            Self::OpenAi | Self::Xai | Self::Anthropic | Self::Custom => {
                AdapterKind::ChatCompletions
            }
        }
    }
}

pub use engine::{CompletionEngine, EngineEvent};
pub use openai::{
    completion_request_plan, endpoint_url, fetch_models, parse_model_ids, test_connection,
    AuthScheme, CompletionProtocol, OpenAiConfig, OpenAiTransport, ProfileRequestConfig,
};
pub use shape::{repair_unclosed_code_completion, shape_suggestion};
pub use snapshot::{take_snapshot, CompletionSnapshot};
pub use state::{CompletionState, GhostSuggestion};
pub use transport::{CancelFlag, CompletionError, NullTransport, Transport};
