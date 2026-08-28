use crate::snapshot::{CompletionMode, CompletionRequest, CompletionSnapshot};
use crate::transport::{CancelFlag, CompletionError, Transport};
use crate::{AdapterKind, ProviderKind};

/// An immutable copy of one profile's connection settings.
///
/// This deliberately does not implement `Debug`: the API key must never make
/// its way into diagnostics, logs, or failed test output.
#[derive(Clone)]
pub struct ProfileRequestConfig {
    pub provider: ProviderKind,
    pub adapter: AdapterKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
    pub allow_http: bool,
}

/// Backwards-compatible name retained for the app crate during the profile
/// migration. It is the same profile-scoped configuration, not a global API
/// configuration.
pub type OpenAiConfig = ProfileRequestConfig;

pub struct OpenAiTransport {
    pub config: ProfileRequestConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionProtocol {
    DeepSeekFim,
    OpenAiChat,
    OpenAiResponses,
    AnthropicMessages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthScheme {
    Bearer,
    AnthropicApiKey,
}

impl AuthScheme {
    pub fn header_name(self) -> &'static str {
        match self {
            Self::Bearer => "authorization",
            Self::AnthropicApiKey => "x-api-key",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestHeader {
    pub name: &'static str,
    pub value: &'static str,
}

pub struct CompletionRequestPlan {
    pub protocol: CompletionProtocol,
    pub endpoint: String,
    pub auth: AuthScheme,
    pub headers: Vec<RequestHeader>,
    pub body: serde_json::Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostMode {
    CurrentLine,
    CurrentStatement,
    Block,
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

pub fn completion_protocol(provider: ProviderKind) -> CompletionProtocol {
    completion_protocol_for(provider, provider.default_adapter())
}

fn completion_protocol_for(provider: ProviderKind, adapter: AdapterKind) -> CompletionProtocol {
    match provider {
        ProviderKind::DeepSeek => match adapter {
            AdapterKind::Fim => CompletionProtocol::DeepSeekFim,
            AdapterKind::ChatCompletions => CompletionProtocol::OpenAiChat,
            AdapterKind::Responses => CompletionProtocol::OpenAiResponses,
        },
        ProviderKind::DeepSeekFim => CompletionProtocol::DeepSeekFim,
        ProviderKind::Xai => CompletionProtocol::OpenAiChat,
        ProviderKind::Anthropic => CompletionProtocol::AnthropicMessages,
        ProviderKind::OpenAi | ProviderKind::Custom => match adapter {
            AdapterKind::Fim | AdapterKind::ChatCompletions => CompletionProtocol::OpenAiChat,
            AdapterKind::Responses => CompletionProtocol::OpenAiResponses,
        },
    }
}

/// Builds a provider-specific completion endpoint. The provider enum, not a
/// host-name heuristic, is the only thing that selects the protocol.
pub fn endpoint_url(provider: ProviderKind, base_url: &str) -> String {
    endpoint_url_for(provider, provider.default_adapter(), base_url)
}

fn endpoint_url_for(provider: ProviderKind, adapter: AdapterKind, base_url: &str) -> String {
    match completion_protocol_for(provider, adapter) {
        CompletionProtocol::DeepSeekFim => format!("{}/beta/completions", origin_url(base_url)),
        CompletionProtocol::OpenAiChat => chat_completion_endpoint(base_url),
        CompletionProtocol::OpenAiResponses => responses_endpoint(base_url),
        CompletionProtocol::AnthropicMessages => format!("{}/v1/messages", origin_url(base_url)),
    }
}

fn model_endpoint_url(config: &ProfileRequestConfig) -> String {
    match completion_protocol_for(config.provider, config.adapter) {
        CompletionProtocol::DeepSeekFim => format!("{}/models", origin_url(&config.base_url)),
        CompletionProtocol::AnthropicMessages => {
            format!("{}/v1/models", origin_url(&config.base_url))
        }
        CompletionProtocol::OpenAiChat | CompletionProtocol::OpenAiResponses => {
            let base = trim_base_url(&config.base_url);
            if base.ends_with("/models") {
                base.to_string()
            } else {
                format!("{}/models", openai_api_root(base))
            }
        }
    }
}

fn trim_base_url(base_url: &str) -> &str {
    base_url.trim().trim_end_matches('/')
}

fn origin_url(base_url: &str) -> String {
    let trimmed = trim_base_url(base_url);
    if let Some((scheme, rest)) = trimmed.split_once("://") {
        let authority = rest.split('/').next().unwrap_or(rest);
        format!("{scheme}://{authority}")
    } else {
        trimmed.to_string()
    }
}

fn chat_completion_endpoint(base_url: &str) -> String {
    let base = trim_base_url(base_url);
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", openai_api_root(base))
    }
}

fn responses_endpoint(base_url: &str) -> String {
    let base = trim_base_url(base_url);
    if base.ends_with("/responses") {
        base.to_string()
    } else {
        format!("{}/responses", openai_api_root(base))
    }
}

fn openai_api_root(base_url: &str) -> &str {
    base_url
        .strip_suffix("/chat/completions")
        .or_else(|| base_url.strip_suffix("/responses"))
        .unwrap_or(base_url)
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
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

pub fn ghost_mode(snapshot: &CompletionSnapshot) -> GhostMode {
    if CompletionMode::from_language(&snapshot.language) != CompletionMode::Code {
        return GhostMode::CurrentLine;
    }

    let last = snapshot.prefix.lines().last().unwrap_or("");
    let trimmed = last.trim_end();
    if trimmed.is_empty()
        || trimmed.ends_with(':')
        || trimmed.ends_with('{')
        || trimmed.ends_with(',')
        || trimmed.ends_with('(')
    {
        GhostMode::Block
    } else {
        GhostMode::CurrentStatement
    }
}

pub fn looks_like_code(prefix: &str) -> bool {
    let last = prefix.lines().last().unwrap_or(prefix);
    last.contains('{')
        || last.contains('}')
        || last.contains(';')
        || last.contains('(')
        || last.contains("def ")
        || last.contains("fn ")
        || last.contains("class ")
        || last.contains("printf")
        || last.contains("#include")
        || prefix.contains("fn ")
        || prefix.contains("def ")
        || prefix.contains("#include")
}

pub fn request_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    let request = CompletionRequest::from_snapshot(snapshot);
    request_body_for_request(CompletionProtocol::OpenAiChat, &request, model)
}

pub fn request_body_for(
    protocol: CompletionProtocol,
    snapshot: &CompletionSnapshot,
    model: &str,
) -> serde_json::Value {
    let request = CompletionRequest::from_snapshot(snapshot);
    request_body_for_request(protocol, &request, model)
}

fn request_body_for_request(
    protocol: CompletionProtocol,
    request: &CompletionRequest,
    model: &str,
) -> serde_json::Value {
    let snapshot = request.context.snapshot();
    match protocol {
        CompletionProtocol::DeepSeekFim => deepseek_fim_body(&snapshot, model),
        CompletionProtocol::OpenAiChat => chat_body_with_context(request, model),
        CompletionProtocol::OpenAiResponses => responses_body_with_context(request, model),
        CompletionProtocol::AnthropicMessages => {
            anthropic_messages_body_with_context(request, model)
        }
    }
}

fn mode_name(mode: crate::snapshot::CompletionMode) -> &'static str {
    match mode {
        crate::snapshot::CompletionMode::Code => "code",
        crate::snapshot::CompletionMode::Markdown => "markdown",
        crate::snapshot::CompletionMode::PlainText => "plain_text",
    }
}

fn typed_cursor_context(request: &CompletionRequest) -> String {
    let context = &request.context;
    format!(
        "LANGUAGE={}\nMODE={}\nCURRENT_LINE:\n{}\nINDENTATION:{:?}\nPREFIX:\n{}\nSUFFIX:\n{}\nCONTINUATION:",
        context.language,
        mode_name(request.mode),
        context.current_line,
        context.indentation,
        context.prefix,
        context.suffix,
    )
}

fn chat_body_with_context(request: &CompletionRequest, model: &str) -> serde_json::Value {
    let mut body = chat_body(&request.context.snapshot(), model);
    body["messages"][0]["content"] = serde_json::Value::String(system_prompt(request.mode).into());
    body["messages"][1]["content"] = serde_json::Value::String(typed_cursor_context(request));
    body
}

fn responses_body_with_context(request: &CompletionRequest, model: &str) -> serde_json::Value {
    let mut body = responses_body(&request.context.snapshot(), model);
    body["instructions"] = serde_json::Value::String(system_prompt(request.mode).into());
    body["input"] = serde_json::Value::String(typed_cursor_context(request));
    body
}

fn anthropic_messages_body_with_context(
    request: &CompletionRequest,
    model: &str,
) -> serde_json::Value {
    let mut body = anthropic_messages_body(&request.context.snapshot(), model);
    body["system"] = serde_json::Value::String(system_prompt(request.mode).into());
    body["messages"][0]["content"] = serde_json::Value::String(typed_cursor_context(request));
    body
}

const CURSOR_SYSTEM_PROMPT: &str = "You are an inline editor completion engine like GitHub Copilot. Emit only the exact characters to insert at the cursor. Continue the current line or current code construct. Never write an article, title, greeting, or new topic. Never invent names, companies, places, or facts that are not already in the prefix. Never explain. Never repeat the prefix.";

const PROSE_SYSTEM_PROMPT: &str = "You are an inline writing completion engine. Emit only the exact characters to insert at the cursor. Continue the current sentence or paragraph in the same language and tone. Never start an article, title, greeting, or new topic. Never invent facts or explain. Never repeat the prefix.";

const PLAIN_TEXT_SYSTEM_PROMPT: &str = "You are an inline plain-text completion engine. Emit only the exact characters to insert at the cursor. Continue the current sentence briefly and naturally in the same language. Never write an article, title, greeting, or explanation. Never repeat the prefix.";

fn system_prompt(mode: crate::snapshot::CompletionMode) -> &'static str {
    match mode {
        crate::snapshot::CompletionMode::Code => CURSOR_SYSTEM_PROMPT,
        crate::snapshot::CompletionMode::Markdown => PROSE_SYSTEM_PROMPT,
        crate::snapshot::CompletionMode::PlainText => PLAIN_TEXT_SYSTEM_PROMPT,
    }
}

fn completion_max_tokens(snapshot: &CompletionSnapshot) -> u64 {
    match ghost_mode(snapshot) {
        GhostMode::CurrentLine => 32,
        GhostMode::CurrentStatement => 48,
        GhostMode::Block => 96,
    }
}

fn cursor_context(snapshot: &CompletionSnapshot) -> String {
    format!(
        "PREFIX:\n{}\nSUFFIX:\n{}\nCONTINUATION:",
        snapshot.prefix, snapshot.suffix
    )
}

fn chat_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "temperature": 0.0,
        "max_tokens": completion_max_tokens(snapshot),
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": CURSOR_SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": cursor_context(snapshot)
            }
        ]
    })
}

fn responses_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "instructions": CURSOR_SYSTEM_PROMPT,
        "input": cursor_context(snapshot),
        "max_output_tokens": completion_max_tokens(snapshot),
        "stream": false
    })
}

fn anthropic_messages_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": completion_max_tokens(snapshot),
        "temperature": 0.0,
        "stream": false,
        "system": CURSOR_SYSTEM_PROMPT,
        "messages": [{
            "role": "user",
            "content": cursor_context(snapshot)
        }]
    })
}

fn deepseek_fim_body(snapshot: &CompletionSnapshot, model: &str) -> serde_json::Value {
    let mode = ghost_mode(snapshot);
    let (max_tokens, stop) = match mode {
        GhostMode::CurrentLine => (32u64, vec!["\n".to_string(), "\n\n".to_string()]),
        GhostMode::CurrentStatement => (48u64, vec!["\n".to_string(), "\n\n".to_string()]),
        GhostMode::Block => (96u64, vec!["\n\n".to_string()]),
    };
    let suffix = if snapshot.suffix.is_empty() {
        "\n".to_string()
    } else {
        snapshot.suffix.clone()
    };
    serde_json::json!({
        "model": model,
        "prompt": snapshot.prefix,
        "suffix": suffix,
        "echo": false,
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "top_p": 1.0,
        "stream": false,
        "stop": stop
    })
}

pub fn completion_request_plan(
    config: &ProfileRequestConfig,
    snapshot: &CompletionSnapshot,
) -> CompletionRequestPlan {
    let request = CompletionRequest::from_snapshot(snapshot);
    completion_request_plan_for_request(config, &request)
}

pub fn completion_request_plan_for_request(
    config: &ProfileRequestConfig,
    request: &CompletionRequest,
) -> CompletionRequestPlan {
    let protocol = completion_protocol_for(config.provider, config.adapter);
    let (auth, headers) = auth_and_headers(protocol);
    CompletionRequestPlan {
        protocol,
        endpoint: endpoint_url_for(config.provider, config.adapter, &config.base_url),
        auth,
        headers,
        body: {
            let mut body = request_body_for_request(protocol, request, &config.model);
            if protocol == CompletionProtocol::OpenAiChat && config.provider == ProviderKind::OpenAi
            {
                if let Some(limit) = body
                    .as_object_mut()
                    .and_then(|object| object.remove("max_tokens"))
                {
                    if let Some(object) = body.as_object_mut() {
                        object.insert("max_completion_tokens".into(), limit);
                    }
                }
            }
            body
        },
    }
}

pub fn streaming_completion_request_plan(
    config: &ProfileRequestConfig,
    snapshot: &CompletionSnapshot,
) -> CompletionRequestPlan {
    let request = CompletionRequest::from_snapshot(snapshot);
    let mut plan = completion_request_plan_for_request(config, &request);
    if let Some(object) = plan.body.as_object_mut() {
        object.insert("stream".into(), serde_json::Value::Bool(true));
    }
    plan
}

fn auth_and_headers(protocol: CompletionProtocol) -> (AuthScheme, Vec<RequestHeader>) {
    match protocol {
        CompletionProtocol::AnthropicMessages => (
            AuthScheme::AnthropicApiKey,
            vec![RequestHeader {
                name: "anthropic-version",
                value: "2023-06-01",
            }],
        ),
        CompletionProtocol::DeepSeekFim
        | CompletionProtocol::OpenAiChat
        | CompletionProtocol::OpenAiResponses => (AuthScheme::Bearer, Vec::new()),
    }
}

pub fn parse_completion_json(body: &str) -> Result<String, CompletionError> {
    parse_completion_json_for(CompletionProtocol::OpenAiChat, body)
}

pub fn parse_stream_chunk(protocol: CompletionProtocol, data: &str) -> Option<String> {
    if data.trim() == "[DONE]" {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    let text = match protocol {
        CompletionProtocol::DeepSeekFim => {
            value.pointer("/choices/0/text").and_then(|v| v.as_str())
        }
        CompletionProtocol::OpenAiChat => value
            .pointer("/choices/0/delta/content")
            .and_then(|v| v.as_str())
            .or_else(|| {
                value
                    .pointer("/choices/0/message/content")
                    .and_then(|v| v.as_str())
            }),
        CompletionProtocol::OpenAiResponses => {
            value.get("delta").and_then(|v| v.as_str()).filter(|_| {
                value.get("type").and_then(|v| v.as_str()) == Some("response.output_text.delta")
            })
        }
        CompletionProtocol::AnthropicMessages => value
            .pointer("/delta/text")
            .and_then(|v| v.as_str())
            .filter(|_| value.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")),
    }?;
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

pub fn parse_sse_body(protocol: CompletionProtocol, body: &str) -> String {
    let mut output = String::new();
    for line in body.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        if let Some(chunk) = parse_stream_chunk(protocol, data.trim_start()) {
            output.push_str(&chunk);
        }
    }
    output
}

pub fn parse_completion_json_for(
    protocol: CompletionProtocol,
    body: &str,
) -> Result<String, CompletionError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| CompletionError::RequestFailed("bad json".into()))?;

    match protocol {
        CompletionProtocol::AnthropicMessages => parse_anthropic_completion(&value),
        CompletionProtocol::OpenAiResponses => parse_responses_completion(&value),
        CompletionProtocol::DeepSeekFim | CompletionProtocol::OpenAiChat => {
            parse_openai_completion(&value)
        }
    }
}

fn parse_openai_completion(value: &serde_json::Value) -> Result<String, CompletionError> {
    if let Some(content) = value.pointer("/choices/0/text").and_then(|v| v.as_str()) {
        if !content.is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(content) = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
    {
        if !content.trim().is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(parts) = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_array())
    {
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
    Err(CompletionError::Empty)
}

fn parse_anthropic_completion(value: &serde_json::Value) -> Result<String, CompletionError> {
    let content = value
        .get("content")
        .and_then(|value| value.as_array())
        .ok_or_else(|| CompletionError::RequestFailed("invalid anthropic response".into()))?;

    content
        .iter()
        .find(|part| part.get("type").and_then(|value| value.as_str()) == Some("text"))
        .and_then(|part| part.get("text").and_then(|value| value.as_str()))
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or(CompletionError::Empty)
}

fn parse_responses_completion(value: &serde_json::Value) -> Result<String, CompletionError> {
    if let Some(text) = value.get("output_text").and_then(|value| value.as_str()) {
        if !text.trim().is_empty() {
            return Ok(text.to_owned());
        }
    }

    let output = value
        .get("output")
        .and_then(|value| value.as_array())
        .ok_or_else(|| CompletionError::RequestFailed("invalid responses response".into()))?;
    let text = output
        .iter()
        .filter_map(|item| item.get("content").and_then(|value| value.as_array()))
        .flatten()
        .filter(|part| part.get("type").and_then(|value| value.as_str()) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .collect::<String>();

    if text.trim().is_empty() {
        Err(CompletionError::Empty)
    } else {
        Ok(text)
    }
}

pub fn parse_model_ids(body: &str) -> Result<Vec<String>, CompletionError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| CompletionError::RequestFailed("bad model json".into()))?;
    let data = value
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| CompletionError::RequestFailed("invalid model response".into()))?;

    let mut ids = Vec::new();
    for entry in data {
        let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let id = id.trim();
        if !id.is_empty() && !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_owned());
        }
    }
    Ok(ids)
}

pub fn classify_status(status: u16) -> CompletionError {
    match status {
        401 | 403 => CompletionError::AuthFailed,
        _ => CompletionError::RequestFailed(format!("http {status}")),
    }
}

fn validate_request_config(
    config: &ProfileRequestConfig,
    require_model: bool,
) -> Result<(), CompletionError> {
    validate_base_url(&config.base_url, config.allow_http)?;
    if config.api_key.trim().is_empty() || (require_model && config.model.trim().is_empty()) {
        return Err(CompletionError::NotConfigured);
    }
    Ok(())
}

fn client_for(config: &ProfileRequestConfig) -> Result<reqwest::blocking::Client, CompletionError> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(
            config.timeout_ms.max(1000),
        ))
        .build()
        .map_err(|error| {
            CompletionError::RequestFailed(redact_api_key(&error.to_string(), &config.api_key))
        })
}

fn redact_api_key(detail: &str, api_key: &str) -> String {
    let detail = if api_key.trim().is_empty() {
        detail.to_owned()
    } else {
        detail.replace(api_key, "[redacted]")
    };
    detail.chars().take(160).collect()
}

fn request_failure(error: reqwest::Error, api_key: &str) -> CompletionError {
    if error.is_timeout() {
        CompletionError::Timeout
    } else {
        CompletionError::RequestFailed(redact_api_key(&error.to_string(), api_key))
    }
}

fn response_text(
    response: reqwest::blocking::Response,
    api_key: &str,
) -> Result<String, CompletionError> {
    let status = response.status().as_u16();
    let is_success = response.status().is_success();
    let body = response.text().map_err(|error| {
        CompletionError::RequestFailed(redact_api_key(&error.to_string(), api_key))
    })?;

    if is_success {
        return Ok(body);
    }

    let detail = redact_api_key(&error_message_from_body(&body), api_key);
    Err(match classify_status(status) {
        CompletionError::AuthFailed => CompletionError::AuthFailed,
        CompletionError::RequestFailed(_) if !detail.is_empty() => {
            CompletionError::RequestFailed(format!("http {status}: {detail}"))
        }
        other => other,
    })
}

fn authorized_request(
    request: reqwest::blocking::RequestBuilder,
    auth: AuthScheme,
    headers: &[RequestHeader],
    api_key: &str,
) -> reqwest::blocking::RequestBuilder {
    let request = match auth {
        AuthScheme::Bearer => request.bearer_auth(api_key),
        AuthScheme::AnthropicApiKey => request.header("x-api-key", api_key),
    };
    headers.iter().fold(request, |request, header| {
        request.header(header.name, header.value)
    })
}

fn execute_completion(
    config: &ProfileRequestConfig,
    snapshot: CompletionSnapshot,
    cancel: CancelFlag,
) -> Result<String, CompletionError> {
    if cancel.is_cancelled() {
        return Err(CompletionError::Cancelled);
    }
    validate_request_config(config, true)?;

    let client = client_for(config)?;
    let plan = completion_request_plan(config, &snapshot);
    let response = authorized_request(
        client.post(&plan.endpoint),
        plan.auth,
        &plan.headers,
        &config.api_key,
    )
    .json(&plan.body)
    .send()
    .map_err(|error| request_failure(error, &config.api_key))?;

    if cancel.is_cancelled() {
        return Err(CompletionError::Cancelled);
    }
    let body = response_text(response, &config.api_key)?;
    parse_completion_json_for(plan.protocol, &body)
}

fn execute_completion_streaming(
    config: &ProfileRequestConfig,
    snapshot: CompletionSnapshot,
    cancel: CancelFlag,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, CompletionError> {
    use std::io::BufRead;

    if cancel.is_cancelled() {
        return Err(CompletionError::Cancelled);
    }
    validate_request_config(config, true)?;

    let client = client_for(config)?;
    let plan = streaming_completion_request_plan(config, &snapshot);
    let response = authorized_request(
        client.post(&plan.endpoint),
        plan.auth,
        &plan.headers,
        &config.api_key,
    )
    .json(&plan.body)
    .send()
    .map_err(|error| request_failure(error, &config.api_key))?;

    if !response.status().is_success() {
        return response_text(response, &config.api_key);
    }

    let mut reader = std::io::BufReader::new(response);
    let mut line = String::new();
    let mut non_stream_body = String::new();
    let mut output = String::new();
    let mut saw_sse = false;

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|error| {
            CompletionError::RequestFailed(redact_api_key(&error.to_string(), &config.api_key))
        })?;
        if bytes == 0 {
            break;
        }
        if cancel.is_cancelled() {
            return Err(CompletionError::Cancelled);
        }

        let Some(data) = line.strip_prefix("data:") else {
            if !saw_sse {
                non_stream_body.push_str(&line);
            }
            continue;
        };
        saw_sse = true;
        let data = data.trim_start();
        if data == "[DONE]" {
            break;
        }
        if let Some(chunk) = parse_stream_chunk(plan.protocol, data) {
            output.push_str(&chunk);
            on_chunk(&chunk);
        }
    }

    if saw_sse {
        if output.is_empty() {
            Err(CompletionError::Empty)
        } else {
            Ok(output)
        }
    } else {
        let result = parse_completion_json_for(plan.protocol, &non_stream_body)?;
        if !result.is_empty() {
            on_chunk(&result);
        }
        Ok(result)
    }
}

pub fn fetch_models(config: &ProfileRequestConfig) -> Result<Vec<String>, CompletionError> {
    validate_request_config(config, false)?;

    let protocol = completion_protocol_for(config.provider, config.adapter);
    let (auth, headers) = auth_and_headers(protocol);
    let client = client_for(config)?;
    let response = authorized_request(
        client.get(model_endpoint_url(config)),
        auth,
        &headers,
        &config.api_key,
    )
    .send()
    .map_err(|error| request_failure(error, &config.api_key))?;
    let body = response_text(response, &config.api_key)?;
    parse_model_ids(&body)
}

fn connection_test_snapshot() -> CompletionSnapshot {
    CompletionSnapshot {
        document_id: 0,
        prefix: "let answer = ".into(),
        suffix: "\n".into(),
        file_name: "connection-test.rs".into(),
        language: "rust".into(),
        generation: 0,
    }
}

pub fn test_connection(config: &ProfileRequestConfig) -> Result<(), CompletionError> {
    execute_completion(config, connection_test_snapshot(), CancelFlag::new()).map(|_| ())
}

impl Transport for OpenAiTransport {
    fn complete(
        &self,
        snapshot: CompletionSnapshot,
        cancel: CancelFlag,
    ) -> Result<String, CompletionError> {
        execute_completion(&self.config, snapshot, cancel)
    }

    fn complete_streaming(
        &self,
        snapshot: CompletionSnapshot,
        cancel: CancelFlag,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<String, CompletionError> {
        execute_completion_streaming(&self.config, snapshot, cancel, on_chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    fn sample_snapshot() -> CompletionSnapshot {
        CompletionSnapshot {
            document_id: 1,
            prefix: "printf(\"hello".into(),
            suffix: String::new(),
            file_name: "main.c".into(),
            language: "c".into(),
            generation: 0,
        }
    }

    fn profile(provider: ProviderKind, base_url: &str) -> ProfileRequestConfig {
        ProfileRequestConfig {
            provider,
            adapter: provider.default_adapter(),
            base_url: base_url.into(),
            api_key: "test-key-not-real".into(),
            model: "test-model".into(),
            timeout_ms: 8_000,
            allow_http: false,
        }
    }

    #[test]
    fn endpoint_appends_chat_completions_once() {
        assert_eq!(
            endpoint_url(ProviderKind::Custom, "https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url(
                ProviderKind::Custom,
                "https://api.example.com/v1/chat/completions",
            ),
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
    fn parse_fim_text_before_chat_content() {
        let body = r#"{"choices":[{"text":" world\")","message":{"content":""}}]}"#;
        assert_eq!(parse_completion_json(body).unwrap(), " world\")");
    }

    #[test]
    fn classify_auth_and_timeout_labels() {
        assert!(matches!(classify_status(401), CompletionError::AuthFailed));
        assert!(matches!(
            classify_status(500),
            CompletionError::RequestFailed(_)
        ));
    }

    #[test]
    fn error_body_uses_openai_message() {
        let body = r#"{"error":{"message":"Insufficient Balance"}}"#;
        assert_eq!(error_message_from_body(body), "Insufficient Balance");
    }

    #[test]
    fn deepseek_root_uses_fim_endpoint() {
        assert_eq!(
            endpoint_url(ProviderKind::DeepSeek, "https://api.deepseek.com"),
            "https://api.deepseek.com/beta/completions"
        );
        assert_eq!(
            endpoint_url(ProviderKind::DeepSeek, "https://api.deepseek.com/v1"),
            "https://api.deepseek.com/beta/completions"
        );
        assert_eq!(
            completion_protocol(ProviderKind::DeepSeek),
            CompletionProtocol::DeepSeekFim
        );
    }

    #[test]
    fn openai_profile_never_uses_deepseek_fim() {
        let config = profile(ProviderKind::OpenAi, "https://api.deepseek.com/v1");
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.protocol, CompletionProtocol::OpenAiChat);
        assert_eq!(
            plan.endpoint,
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(plan.auth, AuthScheme::Bearer);
        assert!(plan.body.get("messages").is_some());
        assert!(plan.body.get("prompt").is_none());
    }

    #[test]
    fn openai_profile_uses_current_chat_completion_token_limit_field() {
        let config = profile(ProviderKind::OpenAi, "https://api.openai.com/v1");
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert!(plan.body.get("max_completion_tokens").is_some());
        assert!(plan.body.get("max_tokens").is_none());
    }

    #[test]
    fn request_plan_carries_typed_editor_context_without_losing_suffix() {
        let mut doc = aitext_core::Document::from_text("def hello():\n    print(");
        doc.set_language(aitext_core::LanguageId::Python);
        doc.set_caret(doc.len_chars());
        let request = crate::snapshot::CompletionRequest::from_document(&doc, 3);
        let config = profile(ProviderKind::OpenAi, "https://api.openai.com/v1");

        let plan = completion_request_plan_for_request(&config, &request);
        let content = plan.body["messages"][1]["content"]
            .as_str()
            .expect("chat context should be text");

        assert!(content.contains("python"));
        assert!(content.contains("    print("));
        assert!(content.contains("def hello():"));
        assert!(content.contains("SUFFIX"));
    }

    #[test]
    fn prose_and_code_requests_use_different_system_prompts() {
        let mut markdown = aitext_core::Document::from_text("# Notes\nWrite a");
        markdown.set_language(aitext_core::LanguageId::Markdown);
        markdown.set_caret(markdown.len_chars());
        let markdown_request = crate::snapshot::CompletionRequest::from_document(&markdown, 0);
        let markdown_plan = completion_request_plan_for_request(
            &profile(ProviderKind::OpenAi, "https://api.openai.com/v1"),
            &markdown_request,
        );
        let markdown_prompt = markdown_plan.body["messages"][0]["content"]
            .as_str()
            .expect("markdown system prompt should be text");

        let mut python = aitext_core::Document::from_text("def hello():\n    return");
        python.set_language(aitext_core::LanguageId::Python);
        python.set_caret(python.len_chars());
        let python_request = crate::snapshot::CompletionRequest::from_document(&python, 0);
        let python_plan = completion_request_plan_for_request(
            &profile(ProviderKind::OpenAi, "https://api.openai.com/v1"),
            &python_request,
        );
        let python_prompt = python_plan.body["messages"][0]["content"]
            .as_str()
            .expect("code system prompt should be text");

        assert!(markdown_prompt.contains("paragraph"));
        assert!(python_prompt.contains("code construct"));
        assert_ne!(markdown_prompt, python_prompt);
    }

    #[test]
    fn ghost_mode_uses_document_language_before_prefix_punctuation() {
        let mut markdown = sample_snapshot();
        markdown.language = "markdown".into();
        markdown.prefix = "A note (with a parenthesis)".into();
        assert_eq!(ghost_mode(&markdown), GhostMode::CurrentLine);

        let mut cpp = sample_snapshot();
        cpp.language = "cpp".into();
        cpp.prefix = "std::cout << \"Hel".into();
        assert_eq!(ghost_mode(&cpp), GhostMode::CurrentStatement);
    }

    #[test]
    fn responses_adapter_uses_responses_endpoint_body_and_parser() {
        let mut config = profile(ProviderKind::OpenAi, "https://api.openai.com/v1");
        config.adapter = AdapterKind::Responses;
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.protocol, CompletionProtocol::OpenAiResponses);
        assert_eq!(plan.endpoint, "https://api.openai.com/v1/responses");
        assert_eq!(plan.auth, AuthScheme::Bearer);
        assert_eq!(plan.body["model"], "test-model");
        assert!(plan.body.get("instructions").is_some());
        assert!(plan.body.get("input").is_some());
        assert!(plan.body.get("max_output_tokens").is_some());
        assert!(plan.body.get("messages").is_none());
        assert!(plan.body.get("max_tokens").is_none());
        assert!(plan.body.get("max_completion_tokens").is_none());

        let response = r#"{
            "output": [{
                "type": "message",
                "content": [{
                    "type": "output_text",
                    "text": " world"
                }]
            }]
        }"#;
        assert_eq!(
            parse_completion_json_for(CompletionProtocol::OpenAiResponses, response).unwrap(),
            " world"
        );
    }

    #[test]
    fn responses_endpoint_is_appended_once_and_models_use_the_api_root() {
        let mut config = profile(ProviderKind::Custom, "https://relay.example/v1/responses");
        config.adapter = AdapterKind::Responses;

        let plan = completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.endpoint, "https://relay.example/v1/responses");
        assert_eq!(
            model_endpoint_url(&config),
            "https://relay.example/v1/models"
        );
    }

    #[test]
    fn fixed_provider_protocols_ignore_responses_adapter() {
        let cases = [
            (ProviderKind::DeepSeekFim, CompletionProtocol::DeepSeekFim),
            (ProviderKind::Xai, CompletionProtocol::OpenAiChat),
            (
                ProviderKind::Anthropic,
                CompletionProtocol::AnthropicMessages,
            ),
        ];

        for (provider, expected) in cases {
            let mut config = profile(provider, "https://provider.example/v1");
            config.adapter = AdapterKind::Responses;
            assert_eq!(
                completion_request_plan(&config, &sample_snapshot()).protocol,
                expected
            );
        }
    }

    #[test]
    fn compatible_profile_keeps_the_widely_supported_max_tokens_field() {
        let config = profile(ProviderKind::Custom, "https://relay.example/v1");
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert!(plan.body.get("max_tokens").is_some());
        assert!(plan.body.get("max_completion_tokens").is_none());
    }

    #[test]
    fn deepseek_profile_uses_fim_independent_of_url_path() {
        let config = profile(ProviderKind::DeepSeek, "https://relay.example/v1/other");
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.protocol, CompletionProtocol::DeepSeekFim);
        assert_eq!(plan.endpoint, "https://relay.example/beta/completions");
        assert_eq!(plan.body["prompt"], "printf(\"hello");
        assert!(plan.body.get("suffix").is_some());
        assert!(plan.body.get("messages").is_none());
    }

    #[test]
    fn deepseek_provider_routes_fim_chat_and_responses_adapters() {
        let cases = [
            (
                AdapterKind::Fim,
                CompletionProtocol::DeepSeekFim,
                "https://api.deepseek.com/beta/completions",
                "prompt",
            ),
            (
                AdapterKind::ChatCompletions,
                CompletionProtocol::OpenAiChat,
                "https://api.deepseek.com/chat/completions",
                "messages",
            ),
            (
                AdapterKind::Responses,
                CompletionProtocol::OpenAiResponses,
                "https://api.deepseek.com/responses",
                "input",
            ),
        ];

        for (adapter, protocol, endpoint, body_key) in cases {
            let mut config = profile(ProviderKind::DeepSeek, "https://api.deepseek.com");
            config.adapter = adapter;
            let plan = completion_request_plan(&config, &sample_snapshot());

            assert_eq!(plan.protocol, protocol);
            assert_eq!(plan.endpoint, endpoint);
            assert!(plan.body.get(body_key).is_some());
        }
    }

    #[test]
    fn anthropic_profile_uses_native_headers_body_and_parser() {
        let config = profile(
            ProviderKind::Anthropic,
            "https://api.anthropic.com/v1/ignored",
        );
        let plan = completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.protocol, CompletionProtocol::AnthropicMessages);
        assert_eq!(plan.endpoint, "https://api.anthropic.com/v1/messages");
        assert_eq!(plan.auth, AuthScheme::AnthropicApiKey);
        assert_eq!(plan.auth.header_name(), "x-api-key");
        assert!(plan
            .headers
            .iter()
            .any(|header| { header.name == "anthropic-version" && header.value == "2023-06-01" }));
        assert!(!plan
            .headers
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case("authorization")));
        assert!(plan.body.get("messages").is_some());
        assert!(plan.body.get("prompt").is_none());

        let response = r#"{"content":[{"type":"text","text":" world"}]}"#;
        assert_eq!(
            parse_completion_json_for(CompletionProtocol::AnthropicMessages, response).unwrap(),
            " world"
        );
    }

    #[test]
    fn model_endpoints_follow_each_provider_api_root() {
        assert_eq!(
            model_endpoint_url(&profile(
                ProviderKind::DeepSeek,
                "https://api.deepseek.com/v1"
            )),
            "https://api.deepseek.com/models"
        );
        assert_eq!(
            model_endpoint_url(&profile(ProviderKind::OpenAi, "https://api.openai.com/v1")),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            model_endpoint_url(&profile(
                ProviderKind::Anthropic,
                "https://api.anthropic.com/v1/ignored"
            )),
            "https://api.anthropic.com/v1/models"
        );
    }

    #[test]
    fn parser_deduplicates_models_in_original_order() {
        let response =
            r#"{"data":[{"id":"gpt-test"},{"id":""},{"id":"gpt-test"},{"id":"claude-test"}]}"#;

        assert_eq!(
            parse_model_ids(response).unwrap(),
            vec!["gpt-test", "claude-test"]
        );
    }

    #[test]
    fn malformed_model_response_does_not_yield_partial_models() {
        let manual_models = vec!["manual-model".to_string()];

        assert!(parse_model_ids(r#"{"data":{}}"#).is_err());
        assert_eq!(manual_models, vec!["manual-model"]);
    }

    #[test]
    fn connection_test_uses_a_fixed_document_independent_snapshot() {
        let snapshot = connection_test_snapshot();

        assert_eq!(snapshot.prefix, "let answer = ");
        assert_eq!(snapshot.suffix, "\n");
        assert_eq!(snapshot.file_name, "connection-test.rs");
        assert_eq!(snapshot.language, "rust");
    }

    #[test]
    fn deepseek_fim_body_uses_prefix_and_implicit_newline_suffix() {
        let body = request_body_for(
            CompletionProtocol::DeepSeekFim,
            &sample_snapshot(),
            "deepseek-v4-flash",
        );
        assert_eq!(body["model"], "deepseek-v4-flash");
        assert_eq!(body["prompt"], "printf(\"hello");
        assert_eq!(body["suffix"], "\n");
        assert_eq!(body["max_tokens"], 48);
        assert!(body.get("messages").is_none());
    }

    #[test]
    fn parses_openai_sse_delta_and_done_event() {
        let event = r#"{"choices":[{"delta":{"content":" world"}}]}"#;
        assert_eq!(
            parse_stream_chunk(CompletionProtocol::OpenAiChat, event),
            Some(" world".into())
        );
        assert_eq!(
            parse_stream_chunk(CompletionProtocol::OpenAiChat, "[DONE]"),
            None
        );
    }

    #[test]
    fn parses_responses_and_anthropic_sse_text_events() {
        let responses = r#"{"type":"response.output_text.delta","delta":" world"}"#;
        let anthropic =
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":" world"}}"#;

        assert_eq!(
            parse_stream_chunk(CompletionProtocol::OpenAiResponses, responses),
            Some(" world".into())
        );
        assert_eq!(
            parse_stream_chunk(CompletionProtocol::AnthropicMessages, anthropic),
            Some(" world".into())
        );
    }

    #[test]
    fn streaming_plan_enables_stream_without_changing_endpoint() {
        let config = profile(ProviderKind::OpenAi, "https://api.openai.com/v1");
        let plan = streaming_completion_request_plan(&config, &sample_snapshot());

        assert_eq!(plan.endpoint, "https://api.openai.com/v1/chat/completions");
        assert_eq!(plan.body["stream"], true);
        assert!(plan.body.get("messages").is_some());
        assert!(plan.body.get("max_completion_tokens").is_some());
    }

    #[test]
    fn sse_body_accumulates_text_events_in_order() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\" hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        assert_eq!(
            parse_sse_body(CompletionProtocol::OpenAiChat, body),
            " hello"
        );
    }

    #[test]
    fn sse_body_ignores_comments_and_malformed_events() {
        let body = concat!(
            ": keep-alive\n\n",
            "data: not-json\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"ok\"}\n\n"
        );
        assert_eq!(
            parse_sse_body(CompletionProtocol::OpenAiResponses, body),
            "ok"
        );
    }

    #[test]
    fn openai_transport_delivers_sse_chunks_before_completion_finishes() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept completion request");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).unwrap();
            let request_text = String::from_utf8_lossy(&request[..bytes_read]);
            assert!(request_text.contains("\"stream\":true"));
            let body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\" hel\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
                "data: [DONE]\n\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let mut config = profile(ProviderKind::Custom, &format!("http://{address}/v1"));
        config.allow_http = true;
        let transport = OpenAiTransport { config };
        let mut chunks = Vec::new();
        let result =
            transport.complete_streaming(sample_snapshot(), CancelFlag::new(), &mut |chunk| {
                chunks.push(chunk.to_string())
            });

        assert_eq!(result.unwrap(), " hello");
        assert_eq!(chunks, vec![" hel", "lo"]);
        server.join().unwrap();
    }

    #[test]
    fn prose_uses_current_line_budget() {
        let snap = CompletionSnapshot {
            document_id: 1,
            prefix: "今天天气怎么样".into(),
            suffix: String::new(),
            file_name: "Untitled-1".into(),
            language: "plain".into(),
            generation: 0,
        };
        assert_eq!(ghost_mode(&snap), GhostMode::CurrentLine);
        let body = request_body_for(CompletionProtocol::DeepSeekFim, &snap, "deepseek-v4-flash");
        assert_eq!(body["max_tokens"], 32);
        assert_eq!(body["suffix"], "\n");
    }
}
