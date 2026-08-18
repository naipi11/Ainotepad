# Aitext Design

Date: 2026-08-18
Status: draft, pending user review
Product: Aitext
Predecessor: helix_copilot (https://github.com/naipi11/helix_copilot)

## 1. Purpose

Aitext is a new Windows-native notepad, not a continuation of the patched Helix tree. It keeps the one useful idea from helix_copilot — inline ghost-text completion — and drops the rest: vendored Helix, the Go LSP proxy, Node, and GitHub Copilot Language Server.

The first release is a light code notepad. A user can open text or source files, edit them quickly, and see a single model completion as gray ghost text at the caret. The user supplies an OpenAI-compatible endpoint. The app stays a single native process.

Success for v1:

- Cold start feels like a notepad, not an IDE or a WebView shell.
- Opening, typing, undoing, finding, and saving stay correct with Chinese IME.
- After a short pause, one ghost suggestion can appear, be accepted with Tab, or be dismissed with Esc.
- Completions work against any user-configured OpenAI-compatible endpoint. HTTPS is the default; plaintext HTTP works only if the user opts in.
- v1 ships as a portable `aitext.exe`. It does not ship a browser engine, Node, an installer, or a language server.

## 2. Non-goals for v1

- File tree, project workspace, plugins, LSP, debugger, terminal
- Multiple carets, minimap, code folding, command palette, session restore
- Overwrite / overtype mode
- Built-in vendor catalog or GitHub Copilot login
- Prompt-engineering UI, chat sidebar, or agent mode
- Regex find/replace, custom proxy UI, cloud sync
- macOS or Linux as a supported target. The code should not be Windows-hostile, but v1 is Windows only.

If a feature is not listed in this spec, it is out of v1.

## 3. Product shape

Aitext is a desktop notepad:

- Multiple tabs
- Open / save / save as
- Undo / redo
- Find / replace
- Line numbers
- Basic syntax highlighting
- Encoding and newline display
- Settings page
- Ghost-text completion from a user-configured OpenAI-compatible API

It is not a terminal editor and not a mini IDE.

## 4. Architecture

One process: `aitext.exe`. UI is drawn with `egui` / `eframe`. There is no Helix runtime, no WebView, no .NET runtime requirement, and no child language server for completions.

The repository is a Cargo workspace with three crates:

| Crate | Responsibility | Forbidden dependencies |
| --- | --- | --- |
| `aitext-core` | Documents, selections, undo, find/replace, encoding, lexing | Network, windowing, secrets |
| `aitext-ai` | Debounce, request generation, OpenAI-compatible client, ghost-text shaping | Windowing, file dialogs |
| `aitext` | Menus, tabs, painting, keybindings, settings, status bar | Model protocol details |

Data flow is one way:

1. Keyboard, mouse, IME commit, and file commands mutate `aitext-core`.
2. After a quiet interval, the shell gives `aitext-ai` a read-only snapshot.
3. `aitext-ai` returns at most one ghost preview for the current generation.
4. Tab writes that preview into the document. Esc drops it. Nothing else mutates the buffer on the model's behalf.

Crashes or API failures never write partial model text into the document. Streaming updates may replace the preview, but only the accepted preview becomes document text.

Offsets in the kernel are Unicode scalar values (Rust `char`s), not bytes and not UTF-16 code units. The painter maps those offsets to glyphs.

```text
+---------------------------+
|           aitext          |
|  menu / tabs / painter    |
|  IME / keys / status bar  |
+------+-------------+------+
       |             |
       v             v
+--------------+  +--------------+
| aitext-core  |  |  aitext-ai   |
| Document     |  | Completion   |
| Workspace    |  | Client       |
| Highlighter  |  | Ghost shaper |
+--------------+  +--------------+
```

## 5. Editor kernel

### 5.1 Document

Each open file is a `Document` with:

- Text stored in a `ropey` rope, addressed by Unicode scalar offset
- One selection: anchor + caret. No multi-cursor
- Undo / redo operations: insert, delete, replace. Consecutive typing coalesces into one undo step
- Identity: optional path, dirty flag, detected newline style, encoding, readonly flag
- Language id chosen from extension or filename, used only for lexing and the completion snapshot

The kernel does not know about tabs, menus, or HTTP.

A new unsaved document is named `Untitled-N`, with `N` incrementing for the process lifetime.

### 5.2 Workspace

`Workspace` is an ordered list of documents plus the current document id. Closing a dirty document is a shell prompt: Save / Don't save / Cancel. v1 does not restore unsaved buffers after restart.

The File menu remembers the last 10 successfully opened or saved paths. It stores paths only, never file contents.

### 5.3 Encoding and newlines

Supported encodings: UTF-8, UTF-8 BOM, UTF-16 LE, UTF-16 BE, GBK / GB18030.

Open:

- Honor BOM when present.
- Otherwise prefer UTF-8.
- If the file is not valid UTF-8, try GBK / GB18030 before giving up.
- Remember the encoding that succeeded and reuse it on save unless the user changes it.

The rope stores the newline characters that were actually decoded. The status bar shows the majority style as `LF` or `CRLF`. Save writes the rope as-is and does not normalize mixed newlines.

### 5.4 Size limit

Limits are measured on the raw file bytes:

- Up to 8 MiB: open editable
- Above 8 MiB and up to 16 MiB: open read-only, no completion
- Above 16 MiB: refuse to open and show a status message

### 5.5 Commands

v1 editing commands:

- Insert text, delete backward/forward
- Arrow keys, Home / End, PageUp / PageDown
- Word movement
- Click to place caret, Shift-click or drag to select, mouse wheel to scroll
- Selection extension with Shift
- Indent / unindent
- Undo / redo
- Cut / copy / paste / select all

Tab without a visible ghost suggestion indents. Tab with a visible ghost suggestion belongs to the completion layer.

v1 is insert mode only. There is no overwrite mode and no INS indicator.

Indent uses the global settings: tabs or spaces, width default 4. v1 does not auto-detect per-file indent and does not keep per-language indent settings.

### 5.6 Find and replace

Find/replace lives in the kernel. The shell only draws the bar.

Supported: find next, find previous, replace current, replace all, match case, whole word. Regex is out of v1.

Search is over the current document only. Match count is shown on the find bar as `n of m`, not on the status bar.

### 5.7 Highlighting

Highlighting is a built-in lexer, not Tree-sitter and not LSP. v1 languages:

- Plain text
- Markdown
- JSON
- TOML
- Rust
- Python
- C / C++
- JavaScript / TypeScript
- PowerShell
- Batch
- INI

Unknown or failed lexing falls back to plain text and never blocks editing. Tokens are ranges plus a small style enum. Line numbers are a view concern; the kernel exposes logical lines only.

### 5.8 IME

IME composition is a first-class state:

- Preedit text may be painted by the shell.
- Preedit text is not in the `Document` and not on the undo stack.
- Completions do not request or paint during composition.
- Only the committed string becomes an insert.

## 6. Shell and UI

The window has four bands and nothing else: menu, tab bar, editor, status bar. No sidebar, no bottom panel, no command palette.

### 6.1 Menu

- File: New, Open, Save, Save As, Close Tab, recent files, Exit
- Edit: Undo, Redo, Cut, Copy, Paste, Select All, Indent
- Find: Find, Replace
- Settings
- Help: About, Keyboard shortcuts

Dropping a file onto the window opens it.

### 6.2 Tabs

Each tab shows the file name and a dirty `*`. Middle click or the close button closes the tab. Too many tabs scroll horizontally; they do not wrap.

### 6.3 Editor surface

The editor is custom-painted, not a system `TextBox`.

- Monospace primary font, default Consolas
- CJK fallback to a system font such as Microsoft YaHei or SimHei
- Gutter line numbers
- Current-line background
- Selection fill
- Blinking caret
- Horizontal and vertical scrollbars
- Word wrap is a setting and defaults to off so line numbers and ghost text stay simple

Whitespace rendering, minimap, and folding are out of v1.

### 6.4 Ghost text

Ghost text is a preview layer, never document text.

Rules:

- Draw only at the caret, in a configurable translucent gray
- One suggestion, never a completion list
- The suggestion may wrap to at most 4 lines
- Caret movement, selection, or a non-prefix edit clears it immediately
- If the user types the exact prefix of the suggestion, trim that prefix and keep the rest
- Tab accepts the remaining suggestion as one undoable insert
- Esc rejects the preview
- No paint and no request while IME composition is active

### 6.5 Find bar

Find/replace is a slim bar above the editor, not a modal dialog. It exposes next / previous, match case, whole word, replace, replace all, and `n of m`.

### 6.6 Status bar

Left to right: `line:column`, encoding, newline style, language, completion state.

Completion state is exactly one of these labels:

- empty
- requesting
- suggested
- not configured
- timeout
- auth failed
- no suggestion
- request failed

Failures do not open a modal. Settings shows the last error as one extra line: HTTP status if any, plus a short reason. That line never includes the API key or file contents.

### 6.7 Keybindings

Windows notepad bindings first:

- `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`
- `Ctrl+W` close tab
- `Ctrl+Tab` / `Ctrl+Shift+Tab` switch tabs
- `Ctrl+Z` / `Ctrl+Y`
- `Ctrl+F` / `Ctrl+H`
- `Ctrl+A`
- `Tab` accept or indent
- `Shift+Tab` unindent
- `Esc` reject ghost text, otherwise close the find bar
- `Ctrl+,` settings

F11 fullscreen is out of v1.

### 6.8 Settings

One settings page:

- Font family and size
- Theme: dark default, light optional
- Word wrap
- Tab width and tab-vs-spaces
- Ghost text enabled, debounce, ghost color
- Base URL, API key, model name, timeout
- Allow plaintext HTTP, default off
- Test connection

The API key field is masked. Test connection sends a tiny `chat/completions` request with `max_tokens = 1` and no document text. It reports success or failure on the settings page, not with a blocking dialog.

Theme tokens cover syntax colors and ghost-text color. v1 does not expose a 30-entry color editor.

## 7. Completion engine

### 7.1 When to request

A document change starts a debounce timer. Default 250 ms. The settings range is 100-800 ms.

Do not request when:

- Base URL, API key, or model name is missing
- Ghost text is disabled
- IME composition is active
- There is a non-empty selection
- The document is readonly
- The document text is larger than 8 MiB
- Completions are in backoff after repeated failures
- The previous request is still in flight; cancel it first, then decide whether the new snapshot should request

### 7.2 Generations

Every request carries a monotonically increasing `generation`. Typing, changing tabs, closing a document, or changing model settings invalidates older generations. Stale responses are discarded and must not paint.

### 7.3 Prompt and payload

The snapshot is truncated, never the whole file by default:

- 4000 characters before the caret
- 500 characters after the caret
- File name and language id

The client calls an OpenAI-compatible `chat/completions` endpoint. Streaming is preferred; non-streaming is the fallback. Temperature is 0.2. Completion length is capped at 120 characters or 4 lines, whichever is hit first. Request timeout defaults to 8 seconds.

The system prompt is fixed and short. It tells the model to continue the text at the caret, without explanation, markdown fences, or repeating the existing prefix. There is no prompt editor in v1.

After the model returns, the engine trims:

- Any prefix already present before the caret
- Anything beyond the line/character cap
- Leading or trailing whitespace that would make the suggestion empty

An empty result is treated as no suggestion.

### 7.4 Local prefix matching

If the user types characters that exactly match the current suggestion prefix, trim locally. Do not fire a new request for that keystroke until debounce expires again. Any other edit, deletion, or caret move kills the suggestion.

Accept inserts the remaining suggestion in one document operation. Reject only clears the preview.

### 7.5 Failure and backoff

Timeouts, HTTP 4xx/5xx, malformed JSON, and empty responses stay quiet. The status bar uses the labels in section 6.6.

Do not retry automatically. After 3 consecutive failures on the same document, wait 5 seconds before another request. Success, changing the endpoint settings, or toggling ghost text off and on clears backoff.

### 7.6 Secrets and network

Config lives under `%APPDATA%\Aitext\`.

- `config.toml` stores base URL, model, debounce, timeout, UI preferences, and recent file paths
- The API key is stored separately and protected with Windows DPAPI
- The key never appears in the git repo, logs, or crash reports
- Logs may include host, status code, duration, and generation, never the bearer token or file contents

Network uses `reqwest` with rustls. HTTPS is required unless the user explicitly allows HTTP. The v1 HTTP client uses the process / system default proxy behavior from environment variables and does not invent a custom proxy UI.

Requests run on background threads. The UI thread never blocks on the network.

## 8. Error handling

| Situation | User-visible result | Document effect |
| --- | --- | --- |
| File missing or unreadable | Status / open dialog error | No document created |
| File above 16 MiB | Status message | Not opened |
| File between 8 and 16 MiB | Status: opened read-only | Read-only document |
| Save failure | Status / save dialog error | Dirty flag remains |
| Invalid encoding on save | Status error | No write |
| Model not configured | Status: not configured | No preview |
| Auth failure | Status: auth failed | No preview |
| Timeout or network error | Status: timeout / request failed | No preview |
| Empty or unusable completion | Status: no suggestion | No preview |
| IME composition | No request | Preedit only in the view |

No modal storm. The only modal confirmations in v1 are dirty-tab close, dirty-exit, and overwrite-on-save conflicts.

## 9. Testing

Tests do not call live model APIs.

`aitext-core`:

- Insert / delete / selection / mouse-equivalent caret placement
- Undo coalescing
- Find next / previous / replace / replace all
- Encoding and newline round trips, including GBK
- Size-limit classification
- Lexer fallback on unknown language

`aitext-ai`:

- Fake transport
- Debounce and cancellation
- Generation invalidation
- Prefix trim after local typing
- Repeated-prefix stripping from model output
- Empty response
- Auth failure
- Stream cancelled mid-flight
- Backoff after three failures

`aitext`:

- Scriptable accept / reject paths
- IME composition suppresses requests
- Settings persist without writing the API key into `config.toml` as plaintext if DPAPI is available

Model quality is not a test.

## 10. Repository and release shape

Suggested layout:

```text
Cargo.toml
crates/aitext-core/
crates/aitext-ai/
crates/aitext/
docs/superpowers/specs/
README.md
```

v1 ships as a portable `aitext.exe` plus a short README. Helix runtime files, `helix-copilot`, and Copilot login commands are not part of this repository.

Licensing: Aitext original source uses MIT. Do not copy Helix or Notepad++ source. Those repositories are behavior references only.

## 11. Implementation order

This spec is the design. The implementation plan is a later step and must follow this order:

1. `aitext-core` document, workspace, undo, find/replace, encodings
2. `aitext` shell that can open, edit, save, and paint a file with line numbers
3. Lexers and status bar metadata
4. Settings and DPAPI-backed secrets
5. `aitext-ai` with a fake transport and ghost-text painting
6. Real OpenAI-compatible client, streaming, backoff
7. IME hardening and packaging

## 12. Decisions already locked

- Product: Windows desktop notepad
- Completion provider: user-supplied OpenAI-compatible API
- Editor depth: light code notepad, not a mini IDE
- Weight: native process, no browser engine
- Stack: Rust + egui/eframe, custom text painter
- One ghost suggestion, Tab / Esc
- Insert mode only
- Portable exe, no installer in v1
- No Copilot LS, no patched Helix, no plugin system in v1
