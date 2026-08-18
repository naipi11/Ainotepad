use crate::snapshot::CompletionSnapshot;
use crate::transport::{CancelFlag, CompletionError, Transport};

#[derive(Clone, Debug)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
    pub allow_http: bool,
}

pub struct OpenAiTransport {
    pub config: OpenAiConfig,
}

pub fn validate_base_url(base_url: &str, allow_http: bool) -> Result<(), CompletionError> {
    if base_url.trim().is_empty() {
        return Err(CompletionError::NotConfigured);
    }
    if base_url.starts_with("http://") {
        if allow_http {
            return Ok(());
        }
        return Err(CompletionError::RequestFailed("https required".into()));
    }
    if base_url.starts_with("https://") {
        return Ok(());
    }
    Err(CompletionError::RequestFailed("unsupported scheme".into()))
}

pub fn endpoint_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if is_bare_openai_compatible_host(trimmed) {
        format!("{trimmed}/v1/chat/completions")
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn is_bare_openai_compatible_host(base_url: &str) -> bool {
    let Some((_, rest)) = base_url.split_once("://") else {
        return false;
    };
    let host_and_path = rest.trim_start_matches('/');
    let (host_port, path) = host_and_path
        .split_once('/')
        .unwrap_or((host_and_path, ""));
    if !path.is_empty() {
        return false;
    }
    let host = host_port
        .split(':')
        .next()
        .unwrap_or(host_port)
        .to_ascii_lowercase();
    matches!(
        host.as_str(),
        "api.deepseek.com" | "api.openai.com" | "api.moonshot.cn"
    )
}

pub fn error_message_from_body(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = value.pointer("/error/message").and_then(|v| v.as_str()) {
            return msg.to_string();
        }
        if let Some(msg) = value.pointer("/message").and_then(|v| v.as_str()) {
            return msg.to_string();
        }
    }
    body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
}

pub fn request_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "temperature": 0.2,
        "max_tokens": 256,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": "You are an inline autocomplete engine. Write only the next few characters to insert at the cursor. Match the language of the existing text. Do not explain. Do not repeat existing text. Do not mention prompts or labels."

            },
            {
                "role": "user",
                "content": format!(
                    "{}",
                    snapshot.prefix
                )
            }
        ]
    })
}

pub fn parse_completion_json(body: &str) -> Result<String, CompletionError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| CompletionError::RequestFailed("bad json".into()))?;
    if let Some(content) = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
    {
        if !content.trim().is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(parts) = value.pointer("/choices/0/message/content").and_then(|v| v.as_array()) {
        let joined = parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
            .collect::<String>();
        if !joined.trim().is_empty() {
            return Ok(joined);
        }
    }
    if let Some(content) = value
        .pointer("/choices/0/delta/content")
        .and_then(|v| v.as_str())
    {
        if !content.trim().is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(content) = value
        .pointer("/choices/0/text")
        .and_then(|v| v.as_str())
    {
        if !content.trim().is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(content) = value
        .pointer("/choices/0/message/reasoning_content")
        .and_then(|v| v.as_str())
    {
        if !content.trim().is_empty() {
            return Ok(content.to_string());
        }
    }
    Err(CompletionError::Empty)
}

pub fn classify_status(status: u16) -> CompletionError {
    match status {
        401 | 403 => CompletionError::AuthFailed,
        _ => CompletionError::RequestFailed(format!("http {status}")),
    }
}

impl Transport for OpenAiTransport {
    fn complete(
        &self,
        snapshot: CompletionSnapshot,
        cancel: CancelFlag,
    ) -> Result<String, CompletionError> {
        if cancel.is_cancelled() {
            return Err(CompletionError::Cancelled);
        }
        validate_base_url(&self.config.base_url, self.config.allow_http)?;
        if self.config.api_key.is_empty() || self.config.model.is_empty() {
            return Err(CompletionError::NotConfigured);
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms.max(1000)))
            .build()
            .map_err(|e| CompletionError::RequestFailed(e.to_string()))?;
        let url = endpoint_url(&self.config.base_url);
        let response = client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&request_body(&snapshot, &self.config.model))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    CompletionError::Timeout
                } else {
                    CompletionError::RequestFailed(e.to_string())
                }
            })?;
        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().unwrap_or_default();
            let detail = error_message_from_body(&body);
            return Err(match classify_status(status) {
                CompletionError::AuthFailed => CompletionError::AuthFailed,
                CompletionError::RequestFailed(_) if !detail.is_empty() => {
                    CompletionError::RequestFailed(format!("http {status}: {detail}"))
                }
                other => other,
            });
        }
        let body = response
            .text()
            .map_err(|e| CompletionError::RequestFailed(e.to_string()))?;
        parse_completion_json(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_appends_chat_completions_once() {
        assert_eq!(
            endpoint_url("https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn http_rejected_unless_allowed() {
        assert!(validate_base_url("http://127.0.0.1:8080/v1", false).is_err());
        assert!(validate_base_url("http://127.0.0.1:8080/v1", true).is_ok());
        assert!(validate_base_url("https://ok.example/v1", false).is_ok());
    }

    #[test]
    fn parse_non_stream_message() {
        let body = r#"{"choices":[{"message":{"content":" world"}}]}"#;
        assert_eq!(parse_completion_json(body).unwrap(), " world");
    }

    #[test]
    fn classify_auth_and_timeout_labels() {
        assert!(matches!(classify_status(401), CompletionError::AuthFailed));
        assert!(matches!(classify_status(500), CompletionError::RequestFailed(_)));
    }

    #[test]
    fn error_body_uses_openai_message() {
        let body = r#"{"error":{"message":"Insufficient Balance"}}"#;
        assert_eq!(error_message_from_body(body), "Insufficient Balance");
    }

    #[test]
    fn deepseek_root_gets_v1_prefix() {
        assert_eq!(
            endpoint_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }
}
