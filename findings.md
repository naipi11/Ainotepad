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
