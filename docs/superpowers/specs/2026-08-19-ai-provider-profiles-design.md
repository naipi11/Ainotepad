# Aitext AI Provider Profiles Design

Date: 2026-08-19
Status: ready for user review
Scope: replaces the single global model endpoint, model list, and API key with independent provider profiles.

## 1. Problem and outcome

The current settings model stores exactly one `base_url`, one `model`, one model list, and one DPAPI key. Selecting a GPT model changes only the model string. If the saved URL is DeepSeek, the transport still selects DeepSeek FIM and sends the GPT name and the DeepSeek credential to the DeepSeek endpoint.

The outcome is a reliable profile switch: choosing a profile changes the URL, provider protocol, credential, selected model, model list, timeout, and HTTP policy together. A stale completion must never survive a profile switch.

## 2. Product boundary

Included:

- Multiple API profiles for DeepSeek, OpenAI, xAI/Grok, Anthropic/Claude, and generic OpenAI-compatible APIs.
- Per-profile model discovery, manual model entry, active-profile switching, connection test, and DPAPI-protected secret storage.
- Provider-specific completion transports for DeepSeek FIM, OpenAI-compatible Chat Completions, and Anthropic Messages.
- Migration from the existing single DeepSeek-style configuration without losing its working key.

Not included in this release:

- Browser cookie reuse, password capture, or emulated ChatGPT/Claude/Grok consumer-account sign-in.
- A fake OAuth button. A future provider login can be added only after a provider publishes a desktop OAuth flow with a registered client ID, redirect URI, PKCE requirements, and terms that permit this use.
- A vendor model catalog that overwrites a user's fetched or manually entered models.
- Multiple concurrent completions or routing one document across several profiles.

## 3. Configuration model

`AppConfig` keeps editor, theme, status-bar, and recent-file settings. Its AI fields become:

```rust
AppConfig {
    profiles: Vec<ApiProfile>,
    active_profile_id: Option<String>,
    // existing editor and appearance fields
}

ApiProfile {
    id: String,
    name: String,
    provider: ProviderKind,
    base_url: String,
    selected_model: String,
    known_models: Vec<String>,
    timeout_ms: u64,
    allow_http: bool,
}

ProviderKind {
    DeepSeekFim,
    OpenAi,
    Xai,
    Anthropic,
    OpenAiCompatible,
}
```

Profile IDs are generated stable IDs, never names, URLs, or model names. Profile names are editable and need only be non-empty after trimming. `known_models` is unique, ordered, and capped at a small practical limit. `active_profile_id` references one profile or is absent when no profile is configured.

The old `base_url`, `model`, and `known_models` fields remain deserializable only for migration. New saves serialize the profile structure and never serialize API keys.

## 4. Secret storage and migration

Secrets remain outside `config.toml`:

```text
%APPDATA%\Aitext\
  config.toml
  api_key.dpapi                 # legacy file retained, never deleted automatically
  secrets\
    <profile-id>.dpapi          # one Windows-DPAPI protected key per profile
```

On the first post-upgrade load, when no profiles exist but a legacy URL, model, or model list exists:

1. Build a profile named `Imported DeepSeek` when the host is `api.deepseek.com`; otherwise `Imported API`.
2. Infer `DeepSeekFim` only for the DeepSeek host; otherwise use `OpenAiCompatible`.
3. Copy the legacy model list, selected model, timeout, and HTTP policy into the profile.
4. If the legacy DPAPI key can be read, store the plaintext only long enough to protect it into `secrets/<profile-id>.dpapi`; do not log it.
5. Retain the old `api_key.dpapi` file until a later explicit cleanup feature.

Invalid or unreadable legacy secrets never stop the application. The imported profile is still created and shown as needing an API key.

## 5. Provider protocol routing

Routing is driven by `ProviderKind`, not host-name guessing. A request receives one immutable `ProfileRequestConfig` copied from the active profile at dispatch time.

| Provider | Completion route | Credential | Result parsing | Model discovery |
| --- | --- | --- | --- | --- |
| DeepSeek FIM | DeepSeek FIM completion endpoint | Bearer API key | `choices[0].text` | provider model-list request; manual fallback |
| OpenAI | OpenAI Chat Completions | Bearer API key | chat message content | `GET /models` |
| xAI | OpenAI-compatible Chat Completions | Bearer API key | chat message content | `GET /models` |
| Anthropic | native Messages API | `x-api-key` plus API-version header | first text content block | Anthropic model-list request |
| Generic compatible | Chat Completions | Bearer API key | chat message content | best-effort `GET /models` |

The DeepSeek FIM payload remains optimized for ghost text. The OpenAI/xAI/generic payload uses the existing concise cursor-continuation prompt. Anthropic receives an equivalent native Messages payload, not an OpenAI-shaped request.

A key regression contract is explicit: an OpenAI, xAI, or generic profile must never select the DeepSeek FIM endpoint merely because another saved profile uses `api.deepseek.com`.

## 6. Model discovery and connection feedback

`Fetch models` runs on a background worker. It captures `(profile_id, configuration_revision)` and reports back only if that exact profile/version is still current; an older response cannot overwrite a newer URL, key, or manual list.

Discovery behavior:

- Normalize the profile URL before forming the vendor's models endpoint.
- Parse vendor model IDs, remove duplicates, and preserve the selected model even when the remote list omits it.
- On success, replace only that profile's discovered list and select the first model only when the profile had no model selected.
- On authentication, network, unsupported-endpoint, or malformed-response failure, preserve the existing list and manual entries. Show a concise profile-scoped recovery message such as `Could not fetch models — add one manually or check URL/key`.

`Test connection` is also profile-scoped. It uses the selected model and a fixed minimal test payload; it never includes document text. The result names the profile and categorizes success, authentication failure, timeout, unsupported model, or HTTP failure without exposing a credential.

## 7. Completion lifecycle

The active profile becomes the sole completion source. On profile switch, model switch, provider change, URL change, or key save:

1. Increment the completion generation.
2. Cancel the in-flight request.
3. Clear the visible ghost preview.
4. Reload only the active profile's DPAPI key.
5. Recompute whether completion is configured.
6. Allow the next editor mutation to queue a request with the new immutable profile snapshot.

This keeps the existing caret/IME guarantees: no request during composition, no ghost text committed automatically, Tab accepts only the current generation, and stale responses are ignored.

The status bar's Model item becomes `Profile name · selected model` when both fit. Completion state and error messages are derived from the active profile, never from a global stale model string.

## 8. Settings surface

The Settings window remains an Operate-mode native tool surface in the existing night-desk manuscript world. The existing close `x`, Close button, and click-away behavior remain.

```text
Profiles                         Active profile
─────────────────────────────    ─────────────────────────────────────
● DeepSeek Flash                 Name       [ DeepSeek Flash        ]
  OpenAI GPT                     Provider   [ DeepSeek FIM        v ]
  Grok                            Base URL   [ https://...           ]
  Claude                          API key    [ •••••••••••••        ]
  Custom relay                    [ Fetch models ] [ Test connection ]
                                 Model      [ deepseek-v4-flash   v ]
[ + Add profile ]                Add model  [                    ] [Add]
                                 Timeout / HTTPS policy / status
```

- The rail is compact, textual, and single-select: a tungsten active rule and dot distinguish the active profile without a heavy card stack.
- `+ Add profile` creates a named generic profile; choosing a provider fills a safe default URL but never writes a key.
- Changing provider, URL, model, key, or HTTP policy marks the profile dirty locally. `Save` persists all profiles and the current active selection.
- `Remove profile` requires an in-window confirmation and removes only that profile's DPAPI secret after confirmation. It cannot remove the last profile without leaving the app in a valid no-profile state.
- UI strings use user-facing terms: `API profile`, `Fetch models`, `Add model manually`, and specific error recovery, not transport-internal jargon.

## 9. Error handling and security

- No API key is included in `Debug`, status messages, request errors, TOML, tests, or logs.
- URLs accept HTTPS by default. HTTP remains an explicit per-profile opt-in.
- If a key is missing, the active profile is visible but completion is `not configured`; editor input remains unaffected.
- A provider error is confined to the active profile and never changes another profile's endpoint or model list.
- Model discovery and completion run away from the egui UI thread.

## 10. Test strategy

Before implementation, add failing tests for these contracts:

1. Legacy single-profile TOML loads into one usable imported profile without putting an API key in `config.toml`.
2. A profile key is saved and reloaded from its own DPAPI path; switching profiles cannot reuse another key.
3. Switching from a DeepSeek profile to an OpenAI profile changes endpoint, headers, request body, and selected model together.
4. An OpenAI profile never resolves to a DeepSeek FIM endpoint.
5. Anthropic uses its own headers/body/parser rather than OpenAI-compatible shapes.
6. Model-list parsers accept representative success responses and preserve manual models after a failed fetch.
7. A stale fetch or completion result cannot alter a profile after its URL/model/profile selection changes.
8. Settings save persists profiles and active selection but never plaintext keys.
9. Profile switching clears the ghost preview and invalidates an in-flight completion.

No test calls live vendor APIs or contains a real API key.

## 11. Delivery order

1. Profile schema, validation, legacy migration, and profile-scoped secret storage.
2. Provider request configuration and regression-tested routing.
3. Model-discovery client/parsers and background result handoff.
4. Profile-switch completion invalidation and status bar integration.
5. Settings profile rail and detail pane.
6. Unit tests, release build, Windows startup check, and visual inspection of the new settings surface.

## 12. Acceptance criteria

- A saved DeepSeek profile still completes through FIM after upgrade.
- An OpenAI/GPT profile sends its own key and URL through its own Chat transport and never reaches DeepSeek.
- xAI and generic OpenAI-compatible endpoints can fetch models or accept manually added ones.
- A Claude profile uses the Messages transport and can be selected independently.
- Switching profiles immediately clears old ghost text, and the next suggestion can only originate from the new profile.
- The app launches when there are no profiles, when a legacy key fails migration, and when model discovery fails.
- The final release build and focused unit tests pass without live network credentials.
