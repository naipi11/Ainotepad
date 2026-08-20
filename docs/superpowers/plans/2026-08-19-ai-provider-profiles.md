# AI Provider Profiles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Replace Aitext's single global AI endpoint, model, and key with safe switchable provider profiles for DeepSeek, OpenAI, xAI, Anthropic, and OpenAI-compatible APIs.

**Architecture:** AppConfig owns serializable ApiProfile records and one active profile ID while DPAPI stores the corresponding keys outside TOML. The AI crate builds provider-directed immutable request plans and parses provider-specific responses; the app captures an active-profile request snapshot, invalidates completion generation on profile changes, and applies background results only when profile ID and revision remain current.

**Tech Stack:** Rust workspace, egui/eframe, serde/toml, reqwest blocking client in worker threads, Windows DPAPI.

**Spec:** docs/superpowers/specs/2026-08-19-ai-provider-profiles-design.md

## Global Constraints

- API keys never appear in config.toml, Debug output, logs, tests, status text, or error text.
- Provider transport selection is based only on ProviderKind; no URL or hostname can override it.
- DeepSeekFim uses /beta/completions; OpenAI, xAI, and generic use Chat Completions; Anthropic uses native /v1/messages.
- A profile switch, profile edit, model edit, URL edit, provider edit, or key save cancels in-flight work, invalidates completion generation, and clears ghost text.
- Model fetch and connection tests run off the egui UI thread and cannot apply stale results.
- Preserve existing close-x, Close button, and click-away settings behavior and the night-desk manuscript identity.
- Do not call a live vendor API in automated tests.

## File Structure

- Modify crates/aitext/src/config.rs: profile schema, legacy TOML migration, profile helpers, and persistence tests.
- Modify crates/aitext/src/secrets.rs: profile-scoped DPAPI paths and legacy-key copy helpers.
- Modify crates/aitext-ai/src/openai.rs and crates/aitext-ai/src/lib.rs: provider-directed completion/model discovery plans, parsing, and HTTP execution.
- Modify crates/aitext-ai/src/engine.rs: explicit completion invalidation API.
- Modify crates/aitext/src/completion.rs and crates/aitext/src/commands.rs: immutable active-profile snapshots, revision guards, background inboxes, and profile activation.
- Modify crates/aitext/src/settings_page.rs: profile rail/detail editor, asynchronous model fetch/test controls, and safe removal confirmation.
- Modify crates/aitext/src/status_bar.rs: profile name plus selected model.
- Modify crates/aitext/src/app.rs only if the settings window needs a bounded layout adjustment; retain all existing close behavior.

---

### Task 1: Persisted profile schema and legacy configuration migration

**Files:**
- Modify: crates/aitext/src/config.rs
- Test: crates/aitext/src/config.rs

**Interfaces:**
- Produce ProviderKind re-exported from aitext_ai, with DeepSeekFim, OpenAi, Xai, Anthropic, and OpenAiCompatible variants serialized as snake_case.
- Produce ApiProfile { id, name, provider, base_url, selected_model, known_models, timeout_ms, allow_http }.
- Produce AppConfig::active_profile(), active_profile_mut(), set_active_profile(&str), add_profile(ApiProfile), remove_profile(&str), and ApiProfile::remember_model(&str).
- Produce AppConfig { profiles: Vec<ApiProfile>, active_profile_id: Option<String>, ...editor preferences } with no public global AI endpoint/model/key fields.

- [ ] **Step 1: Write failing migration and persistence tests**

    Add a legacy_deepseek_config_becomes_imported_profile test that writes legacy top-level base_url, model, known_models, timeout_ms, and allow_http to config.toml, loads it, then asserts Imported DeepSeek, ProviderKind::DeepSeekFim, preserved selected model, preserved timeout, and a populated active profile ID.

    Add a legacy_generic_config_becomes_openai_compatible_profile test with a non-DeepSeek URL and assert Imported API plus ProviderKind::OpenAiCompatible.

    Add a saving_profiles_omits_legacy_top_level_ai_fields test that saves an AppConfig with an OpenAI profile, asserts [[profiles]] is present, asserts base_url/model/known_models are not serialized at TOML top level, and asserts a test key literal is absent.

- [ ] **Step 2: Run the focused tests and verify the expected RED state**

    Run: cargo test -p aitext --lib config::tests::legacy_deepseek_config_becomes_imported_profile -- --exact --test-threads=1

    Expected: FAIL because ApiProfile, ProviderKind, and legacy migration do not exist.

- [ ] **Step 3: Implement profile types and non-serializing legacy fields**

    Define ApiProfile with a generated non-empty stable ID, trimmed non-empty default names, a bounded unique known_models list, and selected_model retained in that list. Define private LegacyAiFields with optional base_url, model, known_models, timeout_ms, and allow_http, flatten it only for deserialization, and skip it during serialization. In AppConfig::clamped, migrate legacy fields only when profiles is empty: use Imported DeepSeek and DeepSeekFim for the api.deepseek.com host, otherwise Imported API and OpenAiCompatible, then set active_profile_id to the created ID.

    Keep a no-profile AppConfig valid. Default new profiles are generic and may have an empty model/key. Imported DeepSeek retains every legacy model value exactly.

- [ ] **Step 4: Run configuration tests**

    Run: cargo test -p aitext --lib config::tests -- --test-threads=1

    Expected: PASS, including legacy migration, duplicate-model normalization, profile selection, and no-plaintext-key persistence tests.

- [ ] **Step 5: Commit the schema change**

    Run:

        git add crates/aitext/src/config.rs
        git commit -m "feat: add persisted AI provider profiles"

### Task 2: Profile-scoped secret storage and non-destructive legacy-key migration

**Files:**
- Modify: crates/aitext/src/secrets.rs
- Modify: crates/aitext/src/commands.rs
- Test: crates/aitext/src/secrets.rs

**Interfaces:**
- Produce profile_secret_path(profile_id: &str) -> PathBuf resolving to config_dir()/secrets/<profile-id>.dpapi.
- Produce store_profile_api_key(profile_id: &str, key: &str), load_profile_api_key(profile_id: &str), remove_profile_api_key(profile_id: &str), and migrate_legacy_api_key(profile_id: &str).
- Retain store_api_key and load_api_key only as explicit legacy compatibility helpers for migration; application code uses profile helpers.

- [ ] **Step 1: Write failing isolation and migration tests**

    Add profile_keys_do_not_cross_between_profiles: store distinct test keys for deepseek and openai, then load both paths and assert that each returns only its own key.

    Add legacy_key_is_copied_without_deleting_legacy_file: write a legacy key, run migrate_legacy_api_key for imported, assert the imported profile reads that key, and assert api_key.dpapi still exists.

    Add removing_one_profile_key_keeps_another_profile_key: remove only one profile key, assert it is missing, and assert the second key remains available.

- [ ] **Step 2: Run the focused tests and verify the expected RED state**

    Run: cargo test -p aitext --lib secrets::tests::profile_keys_do_not_cross_between_profiles -- --exact --test-threads=1

    Expected: FAIL because profile-scoped helpers do not exist.

- [ ] **Step 3: Implement profile paths and migration helpers**

    Create FileSecretStore::at(path: PathBuf). Reject empty or path-unsafe profile IDs before constructing a secret path. Store an empty profile key by deleting only that profile file. migrate_legacy_api_key reads api_key.dpapi, writes only when the profile secret is absent, and leaves api_key.dpapi untouched on success, missing key, or unreadable key.

    At application startup, after load_config has constructed any imported profile, call migrate_legacy_api_key for it and load only the active profile key into AitextApp.api_key.

- [ ] **Step 4: Run secret and startup regression tests**

    Run: cargo test -p aitext --lib secrets::tests commands::tests -- --test-threads=1

    Expected: PASS, covering isolated profile keys, retained legacy secret, and no secret leaked to TOML.

- [ ] **Step 5: Commit the secret migration change**

    Run:

        git add crates/aitext/src/secrets.rs crates/aitext/src/commands.rs
        git commit -m "feat: isolate API keys by provider profile"

### Task 3: Provider-directed completion and model-discovery requests

**Files:**
- Modify: crates/aitext-ai/src/openai.rs
- Modify: crates/aitext-ai/src/lib.rs
- Test: crates/aitext-ai/src/openai.rs

**Interfaces:**
- Produce ProviderKind in aitext-ai so both the app configuration and request layer share the same discriminant.
- Produce ProfileRequestConfig { provider, base_url, api_key, model, timeout_ms, allow_http } without a Debug implementation that could expose a key.
- Produce CompletionProtocol { DeepSeekFim, OpenAiChat, AnthropicMessages }, AuthScheme { Bearer, AnthropicApiKey }, and completion_request_plan(&ProfileRequestConfig, &CompletionSnapshot).
- Produce fetch_models(&ProfileRequestConfig) -> Result<Vec<String>, CompletionError>, test_connection(&ProfileRequestConfig) -> Result<(), CompletionError>, and parse_model_ids(&str) -> Result<Vec<String>, CompletionError>.

- [ ] **Step 1: Write failing protocol-routing tests**

    Add openai_profile_never_uses_deepseek_fim, using an OpenAI ProfileRequestConfig with a DeepSeek-looking URL and asserting the plan protocol is OpenAiChat and endpoint ends in /chat/completions.

    Add deepseek_profile_uses_fim_independent_of_url_path, asserting DeepSeekFim builds origin /beta/completions and emits prompt/suffix rather than messages.

    Add anthropic_profile_uses_native_headers_body_and_parser, asserting endpoint /v1/messages, x-api-key plus anthropic-version headers, no authorization header, a messages payload, and parsing first text content content block.

    Add parser_deduplicates_models_in_original_order and failed_fetch_does_not_mutate_models. The latter exercises the pure parser/request function only; profile mutation remains in app code.

- [ ] **Step 2: Run focused AI tests and verify the expected RED state**

    Run: cargo test -p aitext-ai --lib openai::tests::openai_profile_never_uses_deepseek_fim -- --exact --test-threads=1

    Expected: FAIL because provider-directed config and request plans do not exist.

- [ ] **Step 3: Implement immutable request planning and parsing**

    Replace hostname-driven completion_protocol(base_url) with completion_protocol(provider). Keep URL validation per profile. Build DeepSeek FIM requests with origin /beta/completions, existing FIM body, and Bearer authentication. Build OpenAI, xAI, and generic requests with Chat Completions, existing concise cursor prompt, and Bearer authentication. Build Anthropic requests with native Messages JSON, x-api-key, anthropic-version 2023-06-01, and first text-block response parsing.

    Normalize endpoint roots before appending routes. For chat-compatible endpoints do not append chat/completions twice. For model discovery parse data arrays containing id strings, discard empty IDs, deduplicate while preserving order, and never call a vendor service from a test.

- [ ] **Step 4: Implement blocking worker-compatible execution**

    Make ProviderTransport carry ProfileRequestConfig and call completion_request_plan before reqwest sends. Make fetch_models and test_connection use the same timeout and error categorization as completion. test_connection sends a fixed minimal completion snapshot and never accepts document text as input.

- [ ] **Step 5: Run AI crate tests**

    Run: cargo test -p aitext-ai --lib -- --test-threads=1

    Expected: PASS, including existing suggestion shaping tests and the new provider routing/parsing tests.

- [ ] **Step 6: Commit the provider transport change**

    Run:

        git add crates/aitext-ai/src/openai.rs crates/aitext-ai/src/lib.rs
        git commit -m "feat: route completions by provider profile"

### Task 4: Completion invalidation, profile snapshot dispatch, and stale-result guards

**Files:**
- Modify: crates/aitext-ai/src/engine.rs
- Modify: crates/aitext/src/completion.rs
- Modify: crates/aitext/src/commands.rs
- Test: crates/aitext-ai/src/engine.rs
- Test: crates/aitext/src/completion.rs

**Interfaces:**
- Produce CompletionEngine::invalidate(&mut self) that increments generation, clears pending/inflight/suggestion state, and resets the visible completion state without accepting stale replies.
- Produce AitextApp::activate_profile(&str), active_request_config(), profile_changed(), and profile_revision: u64.
- Completion inbox payloads carry profile_id, profile_revision, completion_generation, and Result<String, CompletionError>.

- [ ] **Step 1: Write failing invalidation tests**

    Add engine_invalidate_makes_old_snapshot_stale: queue a snapshot, record generation, invalidate, then assert a response tagged with the prior generation is ignored and no ghost remains.

    Add switching_profile_clears_ghost_and_reloads_only_new_key: configure two profiles with isolated test secrets, make a visible suggestion, activate the second profile, assert ghost is absent, active key is second only, and completion's generation advanced.

    Add stale_worker_result_cannot_apply_after_profile_revision_changes: construct a completion inbox item for revision N, increment to N+1 via a profile mutation, drain the item, and assert it cannot create a suggestion or alter status.

- [ ] **Step 2: Run focused invalidation tests and verify the expected RED state**

    Run: cargo test -p aitext-ai --lib engine::tests::engine_invalidate_makes_old_snapshot_stale -- --exact --test-threads=1

    Expected: FAIL because CompletionEngine::invalidate is missing.

- [ ] **Step 3: Implement completion generation invalidation**

    Implement invalidate by cancelling the state represented by pending/inflight, clearing suggestion and last error as appropriate, incrementing the generation counter, and setting completion state to Empty or NotConfigured according to existing configured state. Keep reject for user Esc semantics; use invalidate for source/config changes.

- [ ] **Step 4: Capture active profile atomically for every worker**

    Make refresh_completion_config read only config.active_profile() plus the currently loaded matching key. Make start_completion_request copy ProfileRequestConfig, current profile ID, revision, snapshot generation, and a fresh CancelFlag into the worker. At profile activation and every saved profile field change, cancel the current request, call engine.invalidate, clear inbox/inflight, clear pending API-key UI text, reload only the active profile secret, increment profile_revision, and refresh configured state.

- [ ] **Step 5: Guard completion responses**

    In drain_completion, apply a worker result only when profile ID equals the current active ID, result revision equals profile_revision, and completion generation equals engine.generation. Drop every other result silently. Preserve existing IME handling, Tab acceptance, Esc rejection, and empty-prefix ghost clearing.

- [ ] **Step 6: Run completion and engine test suites**

    Run: cargo test -p aitext-ai --lib engine::tests -- --test-threads=1

    Run: cargo test -p aitext --lib completion::tests commands::tests -- --test-threads=1

    Expected: PASS, including IME and stale ghost regressions.

- [ ] **Step 7: Commit lifecycle isolation**

    Run:

        git add crates/aitext-ai/src/engine.rs crates/aitext/src/completion.rs crates/aitext/src/commands.rs
        git commit -m "feat: invalidate completions on profile changes"

### Task 5: Profile management settings surface, asynchronous operations, and profile-aware status

**Files:**
- Modify: crates/aitext/src/settings_page.rs
- Modify: crates/aitext/src/status_bar.rs
- Modify: crates/aitext/src/app.rs when a bounded settings size or scroll adjustment is required
- Modify: crates/aitext/src/commands.rs or crates/aitext/src/completion.rs for model/test worker inbox ownership
- Test: crates/aitext/src/settings_page.rs
- Test: crates/aitext/src/status_bar.rs

**Interfaces:**
- Produce ProfileWorkerResult { profile_id, profile_revision, operation, result } and a non-blocking receiver polled by the app update loop.
- Produce AitextApp::fetch_active_profile_models(), test_active_profile_connection(), and apply_profile_worker_result(...).
- Produce a compact profile rail and detail form with Add profile, Fetch models, Test connection, Add model manually, Save changes, and Remove profile controls.
- Produce status text Profile name · selected model, or profile unset when no active profile exists.

- [ ] **Step 1: Write failing app behavior tests**

    Add save_settings_persists_active_profile_without_key: set up an OpenAI profile, enter a test key in the profile-specific pending key field, save, assert config.toml includes profile name/model but not key, and assert only that profile secret contains the key.

    Add stale_model_fetch_result_does_not_overwrite_changed_profile: create a profile with a manual model, capture revision, mutate its URL/model and increment revision, apply a successful old fetch result, then assert the manual model and current selected model remain unchanged.

    Add status_bar_identifies_active_profile_and_model: select a profile named Grok with model grok-test, then assert the Model status text equals Grok · grok-test.

- [ ] **Step 2: Run focused UI-state tests and verify the expected RED state**

    Run: cargo test -p aitext --lib settings_page::tests::stale_model_fetch_result_does_not_overwrite_changed_profile -- --exact --test-threads=1

    Expected: FAIL because profile workers and profile-specific settings persistence do not exist.

- [ ] **Step 3: Implement background model discovery and connection feedback**

    On Fetch models or Test connection, copy the active profile's immutable ProfileRequestConfig, active ID, and profile_revision, then spawn a worker that sends a ProfileWorkerResult through an mpsc channel. Do not mutate configuration from the worker. When receiving a current Fetch models success, replace only that profile's known_models with normalized remote IDs while retaining a non-empty selected model even if omitted. When receiving any failure, preserve known_models and manual entries and set a concise profile-scoped message such as Could not fetch models — add one manually or check URL/key. For connection tests send only the selected model plus fixed minimal request data.

- [ ] **Step 4: Load the native UI craft floor before editing the settings surface**

    Run: Get-Content -Raw C:\Users\33384\.codex\skills\impeccable\reference\craft-floor.md

    Apply the existing night-desk manuscript design: warm dark paper, tungsten accent, YaHei-first UI type, thin rules, and compact textual control density. Do not introduce card dashboards, gradients, or a web-style sidebar.

- [ ] **Step 5: Replace global model controls with a profile rail and active detail pane**

    Render a left rail titled API profiles. Use one compact selectable text row per profile with a tungsten active rule and dot, then a + Add profile button that inserts a named OpenAICompatible profile and activates it. In the detail pane expose editable Name, Provider, Base URL, masked API key, Fetch models, Test connection, selected Model combo, Add model manually, Timeout, Allow plaintext HTTP, Save changes, and Remove profile.

    Provider changes assign only safe default URLs when the current URL is blank; never add a key. Selecting a profile calls activate_profile immediately. Text-field response changes call profile_changed so stale workers and ghost text cannot survive unsaved edits. Removing a profile opens an in-window confirmation, deletes only its own DPAPI secret after confirmation, updates active selection to a remaining profile or None, and leaves the app valid with no profiles.

    Preserve close x, bottom Close, and click-away behavior implemented in app.rs. Keep setting controls keyboard-accessible and labels explicit; show saved-key state without rendering the key.

- [ ] **Step 6: Update profile-aware status text and app polling**

    Change StatusItem::Model so it gets config.active_profile and returns profile.name plus selected_model joined by ·. Let the app update loop drain profile worker results beside completion responses. Keep status entries user-configurable and keep completion/error text scoped to the active profile.

- [ ] **Step 7: Run application tests and bounded visual validation**

    Run: cargo test -p aitext --lib -- --test-threads=1

    Run: cargo build -p aitext

    Start the debug executable, open Settings, switch profiles, add a generic profile, select a provider, verify no previous ghost text remains, and verify close x, Close, and click-away all dismiss the window.

- [ ] **Step 8: Commit the settings surface**

    Run:

        git add crates/aitext/src/settings_page.rs crates/aitext/src/status_bar.rs crates/aitext/src/app.rs crates/aitext/src/commands.rs crates/aitext/src/completion.rs
        git commit -m "feat: manage API provider profiles in settings"

### Task 6: Release verification and Windows handoff

**Files:**
- Verify: Cargo.lock
- Verify: crates/aitext-ai/src/engine.rs
- Verify: crates/aitext-ai/src/openai.rs
- Verify: crates/aitext/src/config.rs
- Verify: crates/aitext/src/secrets.rs
- Verify: crates/aitext/src/completion.rs
- Verify: crates/aitext/src/settings_page.rs
- Verify: crates/aitext/src/status_bar.rs

**Interfaces:**
- Delivers a release executable that starts without a console window and supports no-profile startup, migrated DeepSeek, OpenAI-compatible, xAI, Anthropic, and generic profile selection.

- [ ] **Step 1: Run focused crate test suites serially**

    Run: cargo test -p aitext-core --lib -- --test-threads=1

    Run: cargo test -p aitext-ai --lib -- --test-threads=1

    Run: cargo test -p aitext --lib -- --test-threads=1

    Expected: PASS with no live credentials, no live HTTP requests, and no plaintext secret output.

- [ ] **Step 2: Run full release build**

    Run: cargo build --release -p aitext

    Expected: PASS and create target/release/aitext.exe.

- [ ] **Step 3: Run a bounded Windows startup smoke test**

    Run: Start-Process -FilePath .\target\release\aitext.exe -WindowStyle Hidden

    Confirm that the application opens, no command prompt appears, and it can open Settings with zero profiles and a migrated config.

- [ ] **Step 4: Review the final diff without disturbing pre-existing work**

    Run: git diff --check

    Run: git status --short

    Compare the resulting diff only against the known initial dirty-file list. Do not reset, checkout, discard, or broadly stage user changes. If pre-existing edits overlap files changed by this feature, report the overlap and leave commits to the user unless each staged hunk can be shown to be feature-only.
