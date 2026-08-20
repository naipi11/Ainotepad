# Findings

- Current user-facing strings are distributed across `app.rs`, `settings_page.rs`, `find_bar.rs`, `editor_view.rs`, command/status paths, and help dialogs; ad hoc replacement would preserve mixed language.
- `AppConfig` has no language field, so a persisted three-state preference and migration default are required.
- The approved reference establishes a stable black shell, white-paper contrast, blue navigation, green completion, and a single orange manuscript rule.
- A permanent IDE sidebar would conflict with Aitext's lightweight Notepad positioning; the design keeps the editor full-width.
- Existing themes should control the editor surface while the Paper Cut shell remains structurally stable.

## 2026-08-20 — Formal Aitext release preparation

- The GitHub target is public `naipi11/helix_copilot`, default branch `main`, with old Helix Copilot tags/releases `v0.1.0` through `v0.2.6`.
- The local checkout is `codex/ai-provider-profiles` with no configured Git remote and a divergent local history; the user selected destructive restart strategy B.
- Release identity is `v0.1.0`, unsigned Windows installer, Portable ZIP, and SHA256SUMS.
- No local Inno Setup/WiX/NSIS compiler is installed. GitHub Actions will install/use Inno Setup on `windows-latest` and produce the installer.
- GitHub CLI is authenticated as `naipi11` with `repo` and `workflow` scopes. No repository code-signing secrets exist.
- README will use `README.md` (English) and `README.zh-CN.md` (Simplified Chinese), with explicit language links rather than JavaScript toggling so GitHub renders both reliably.
- Paper Cut is the approved visual direction. Planned media: exact-language PNG hero variants and exact-language animated GIF demos with a green ghost-text reveal; no desktop automation or real API key is needed.
- Typed translation keys provide compile-time completeness without runtime translation files or extra parsing.

## 2026-08-20 — v0.1.0 publication evidence

- RC workflow `32371624434` passed on the temporary branch before destructive repository changes; its hosted Windows installer and portable ZIP were downloaded and hash-checked without launching the installer.
- Repository was renamed to `naipi11/Aitext`; description/topics were updated; old v0.x releases and tags were removed; `main` now points to release commit `ca4f7b4526cb97b33f295086af1a1a4bbaa4efcd`.
- Formal tag workflow `32372786776` passed in 6m0s. GitHub Release `v0.1.0` is published and non-draft with installer, portable ZIP, and SHA256SUMS assets.
- Final downloaded assets were verified locally. The installer is `NotSigned` by design; the portable archive contains only `aitext.exe`, `LICENSE`, `README.md`, and `README.zh-CN.md`. The installer was not run.
- The initial broad old-release deletion loop stopped safely when it reached tag `v0.1.1`, which had no GitHub Release. Cleanup was corrected to delete only the two actual old Releases (`v0.2.5`, `v0.2.6`) and then delete all old tag refs exactly; verification showed only `v0.1.0` remains.
- The first multi-input `ffprobe` invocation was invalid because `ffprobe` accepts one input per command. It was corrected to four individual probes; all PNG/GIF metadata checks passed.

## 2026-08-21 — Ainotepad implementation evidence

- Markdown is now the default language for untitled documents; unknown and extensionless files fall back to Markdown while plain-text extensions remain Plain Text.
- Added C#, HTML, and CSS lexer routing with coverage and unterminated-input tests. The renamed completion snapshot maps all new language IDs to stable context names.
- Replaced per-character editor painting with one egui Galley per line. Mixed Chinese/Latin text, caret, selection, IME preedit, and ghost text now use shared measured layout coordinates.
- Added a localized status-bar document-type selector with Text/Programming groups and an Edit-menu access path. Selecting a type preserves document text/caret and invalidates stale completion context.
- Renamed the local Cargo packages and executable to Ainotepad/ainotepad.exe. Config migration targets LOCALAPPDATA/Ainotepad and reads legacy APPDATA or LOCALAPPDATA Aitext directories without deleting them.
- Updated current README, product/design docs, Inno/workflows, release notes, and bilingual showcase media. Current media uses Ainotepad branding and 32/28-frame two-stage sentence/code reveal GIFs.
- Local validation: workspace tests 105 app + 37 AI + 38 core passed; cargo fmt check passed; cargo build --release -p ainotepad passed; only the four pre-existing unused default-config helper warnings remain.
