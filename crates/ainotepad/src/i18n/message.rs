use ainotepad_ai::CompletionError;

use super::Locale;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    IncompleteProfile,
    Timeout,
    Authentication,
    ModelUnavailable,
    Http,
    Empty,
    Cancelled,
    Request,
}

impl FailureReason {
    pub fn from_completion_error(error: &CompletionError) -> Self {
        match error {
            CompletionError::NotConfigured => Self::IncompleteProfile,
            CompletionError::Timeout => Self::Timeout,
            CompletionError::AuthFailed => Self::Authentication,
            CompletionError::Empty => Self::Empty,
            CompletionError::Cancelled => Self::Cancelled,
            CompletionError::RequestFailed(detail) if looks_like_model_error(detail) => {
                Self::ModelUnavailable
            }
            CompletionError::RequestFailed(detail)
                if detail
                    .trim_start()
                    .to_ascii_lowercase()
                    .starts_with("http ") =>
            {
                Self::Http
            }
            CompletionError::RequestFailed(_) => Self::Request,
        }
    }

    pub fn render(self, locale: Locale) -> &'static str {
        match (locale, self) {
            (Locale::En, Self::IncompleteProfile) => "profile is incomplete",
            (Locale::En, Self::Timeout) => "request timed out",
            (Locale::En, Self::Authentication) => "authentication failed",
            (Locale::En, Self::ModelUnavailable) => "selected model is unavailable",
            (Locale::En, Self::Http) => "provider returned an HTTP error",
            (Locale::En, Self::Empty) => "provider returned no content",
            (Locale::En, Self::Cancelled) => "request was cancelled",
            (Locale::En, Self::Request) => "request failed",
            (Locale::ZhCn, Self::IncompleteProfile) => "配置不完整",
            (Locale::ZhCn, Self::Timeout) => "请求超时",
            (Locale::ZhCn, Self::Authentication) => "身份验证失败",
            (Locale::ZhCn, Self::ModelUnavailable) => "所选模型不可用",
            (Locale::ZhCn, Self::Http) => "服务商返回 HTTP 错误",
            (Locale::ZhCn, Self::Empty) => "服务商未返回内容",
            (Locale::ZhCn, Self::Cancelled) => "请求已取消",
            (Locale::ZhCn, Self::Request) => "请求失败",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiMessage {
    UnsavedChanges,
    OpenFailed(String),
    SaveFailed(String),
    EncodeFailed(String),
    SettingsSaved,
    NewProfileAdded,
    ProfileRemoved,
    DraftApiKeyChanged,
    ApiKeyCleared,
    FetchModelsNeedsUrlKey,
    FetchingModels {
        profile: String,
    },
    FetchedModels {
        count: usize,
        profile: String,
    },
    FetchModelsFailed {
        profile: String,
        reason: FailureReason,
    },
    ConnectionNeedsUrlModelKey,
    TestingConnection {
        profile: String,
    },
    ConnectionVerified {
        profile: String,
    },
    ConnectionFailed {
        profile: String,
        reason: FailureReason,
    },
    NoSuggestion,
    CompletionDetail(String),
}

impl UiMessage {
    pub fn render(&self, locale: Locale) -> String {
        match (locale, self) {
            (Locale::En, Self::UnsavedChanges) => "Unsaved changes".into(),
            (Locale::ZhCn, Self::UnsavedChanges) => "有未保存的更改".into(),
            (Locale::En, Self::OpenFailed(detail)) => format!("Open failed: {detail}"),
            (Locale::ZhCn, Self::OpenFailed(detail)) => format!("打开失败：{detail}"),
            (Locale::En, Self::SaveFailed(detail)) => format!("Save failed: {detail}"),
            (Locale::ZhCn, Self::SaveFailed(detail)) => format!("保存失败：{detail}"),
            (Locale::En, Self::EncodeFailed(detail)) => {
                format!("Encoding failed: {detail}")
            }
            (Locale::ZhCn, Self::EncodeFailed(detail)) => {
                format!("编码失败：{detail}")
            }
            (Locale::En, Self::SettingsSaved) => "Settings saved.".into(),
            (Locale::ZhCn, Self::SettingsSaved) => "设置已保存。".into(),
            (Locale::En, Self::NewProfileAdded) => "New profile added.".into(),
            (Locale::ZhCn, Self::NewProfileAdded) => "已添加新配置。".into(),
            (Locale::En, Self::ProfileRemoved) => "Profile removed.".into(),
            (Locale::ZhCn, Self::ProfileRemoved) => "配置已移除。".into(),
            (Locale::En, Self::DraftApiKeyChanged) => {
                "Draft API key changed; save to store it.".into()
            }
            (Locale::ZhCn, Self::DraftApiKeyChanged) => {
                "API 密钥草稿已更改；保存设置后才会存储。".into()
            }
            (Locale::En, Self::ApiKeyCleared) => {
                "API key cleared from this session; save to remove it.".into()
            }
            (Locale::ZhCn, Self::ApiKeyCleared) => {
                "当前会话中的 API 密钥已清除；保存设置后才会删除。".into()
            }
            (Locale::En, Self::FetchModelsNeedsUrlKey) => {
                "Fetch models needs a URL and API key.".into()
            }
            (Locale::ZhCn, Self::FetchModelsNeedsUrlKey) => "获取模型需要 URL 和 API 密钥。".into(),
            (Locale::En, Self::FetchingModels { profile }) => {
                format!("Fetching models for {profile}…")
            }
            (Locale::ZhCn, Self::FetchingModels { profile }) => {
                format!("正在为 {profile} 获取模型…")
            }
            (Locale::En, Self::FetchedModels { count, profile }) if *count == 1 => {
                format!("Fetched 1 model for {profile}.")
            }
            (Locale::En, Self::FetchedModels { count, profile }) => {
                format!("Fetched {count} models for {profile}.")
            }
            (Locale::ZhCn, Self::FetchedModels { count, profile }) => {
                format!("已为 {profile} 获取 {count} 个模型。")
            }
            (Locale::En, Self::FetchModelsFailed { profile, reason }) => {
                format!(
                    "Could not fetch models for {profile}: {}.",
                    reason.render(locale)
                )
            }
            (Locale::ZhCn, Self::FetchModelsFailed { profile, reason }) => {
                format!("无法为 {profile} 获取模型：{}。", reason.render(locale))
            }
            (Locale::En, Self::ConnectionNeedsUrlModelKey) => {
                "Connection test needs a URL, model, and API key.".into()
            }
            (Locale::ZhCn, Self::ConnectionNeedsUrlModelKey) => {
                "连接测试需要 URL、模型和 API 密钥。".into()
            }
            (Locale::En, Self::TestingConnection { profile }) => {
                format!("Testing connection for {profile}…")
            }
            (Locale::ZhCn, Self::TestingConnection { profile }) => {
                format!("正在测试 {profile} 的连接…")
            }
            (Locale::En, Self::ConnectionVerified { profile }) => {
                format!("Connection verified for {profile}.")
            }
            (Locale::ZhCn, Self::ConnectionVerified { profile }) => {
                format!("{profile} 连接验证成功。")
            }
            (Locale::En, Self::ConnectionFailed { profile, reason }) => {
                format!(
                    "Connection failed for {profile}: {}.",
                    reason.render(locale)
                )
            }
            (Locale::ZhCn, Self::ConnectionFailed { profile, reason }) => {
                format!("{profile} 连接失败：{}。", reason.render(locale))
            }
            (Locale::En, Self::NoSuggestion) => "No suggestion".into(),
            (Locale::ZhCn, Self::NoSuggestion) => "无补全建议".into(),
            (Locale::En, Self::CompletionDetail(detail)) => {
                format!("Completion failed: {detail}")
            }
            (Locale::ZhCn, Self::CompletionDetail(detail)) => {
                format!("补全失败：{detail}")
            }
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::OpenFailed(_)
                | Self::SaveFailed(_)
                | Self::EncodeFailed(_)
                | Self::FetchModelsFailed { .. }
                | Self::ConnectionFailed { .. }
                | Self::CompletionDetail(_)
        )
    }

    pub fn is_completion_feedback(&self) -> bool {
        matches!(self, Self::NoSuggestion | Self::CompletionDetail(_))
    }
}

fn looks_like_model_error(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    detail.contains("model")
        && [
            "not found",
            "unsupported",
            "unavailable",
            "does not exist",
            "invalid model",
        ]
        .iter()
        .any(|marker| detail.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_messages_render_complete_sentences_in_each_locale() {
        let message = UiMessage::FetchedModels {
            count: 3,
            profile: "DeepSeek".into(),
        };
        assert_eq!(message.render(Locale::En), "Fetched 3 models for DeepSeek.");
        assert_eq!(
            message.render(Locale::ZhCn),
            "已为 DeepSeek 获取 3 个模型。"
        );
    }

    #[test]
    fn technical_error_detail_is_preserved_inside_localized_context() {
        let message = UiMessage::OpenFailed("access denied".into());
        assert_eq!(message.render(Locale::En), "Open failed: access denied");
        assert_eq!(message.render(Locale::ZhCn), "打开失败：access denied");
    }

    #[test]
    fn authentication_reason_is_localized() {
        assert_eq!(
            FailureReason::Authentication.render(Locale::En),
            "authentication failed"
        );
        assert_eq!(
            FailureReason::Authentication.render(Locale::ZhCn),
            "身份验证失败"
        );
    }
}
