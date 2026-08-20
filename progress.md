# Progress

## 2026-08-20 — Paper Cut and bilingual UI

- User selected three language modes: Follow Windows, Simplified Chinese, and English.
- User approved the Paper Cut visual direction derived from the generated hero.
- Inspected current application shell, Settings, configuration, and distributed interface strings.
- Defined the stable shell, theme interaction, typed translation architecture, locale resolution, migration, tests, and acceptance criteria.
- Wrote the design specification for user review; no implementation code changed and no commit was created.
- Self-review found no placeholders or contradictory scope; clarified unknown-language fallback and intentional language-option exceptions.
- Cleaned up the superseded README demo process state and its isolated temporary configuration directory; retained the approved hero in `docs/media/` as the visual reference.
- User approved the written Paper Cut and bilingual UI specification.
- Mapped implementation into locale/config, translation catalog/messages, localized surfaces, Settings language selector, Paper Cut shell, and final Windows verification tasks.
- Wrote the six-task test-driven implementation plan and self-reviewed it against all ten spec sections.
- Placeholder scan is clean, interfaces are consistent across tasks, and `git diff --check` passes for the planning artifacts.
- No implementation code, staging, commit, or push was performed in the planning turn.
- User selected Inline Execution.
- Verified the existing `codex/ai-provider-profiles` working tree in place; creating a new worktree would omit required unstaged integration changes.
- Fresh baseline: 135 tests passed, 0 failed; 4 existing dead-code warnings remain.
- Task 1 RED verified missing locale types and missing config field; GREEN now passes 3 locale tests and 16 configuration tests.
- Enabled only the existing Windows crate's `Win32_Globalization` feature; no new runtime dependency was added.
- Task 2 RED verified the absent catalog, completion mapping, and dynamic message types; GREEN passes 10 i18n tests.
- Static copy is compile-time typed; dynamic messages preserve technical details inside localized context.
- Task 3 localized menus, recent files, generated untitled tabs, Help, Find/Replace, empty editor state, completion state, model fallback, and typed status messages.
- Task 3 GREEN passes all 86 `aitext` library tests; language switching preserves document, ghost text, profile revision, and completion generation.
- Task 4 localized every Settings section, profile/provider/adapter control, appearance control, status-bar option, helper, confirmation, and footer action.
- Added the immediate Follow Windows / 简体中文 / English selector; all 25 Settings tests pass, including unsaved-draft isolation and DPAPI-only key persistence.
- Re-loaded the approved `frontend-design` and `impeccable` design guidance before Paper Cut implementation; the approved visual spec remains the pinned direction.
- Task 5 introduced stable `ShellColors`, theme-adjusted green ghost text, the permanent orange manuscript rule, shell-driven egui visuals, and a 2 px blue active-tab rule.
- Settings and all transient tool surfaces now use Paper Cut shell roles instead of editor-theme chrome; 93 application library tests passed before final formatting.
- Source scan found only the two intentional `x` close marks and the technical `https://…` URL hint outside the typed catalog.
- Updated PRODUCT.md and DESIGN.md to record the shipped language and Paper Cut decisions; no staging, commit, or push occurred.
- Full final workspace suite: 163 tests passed, 0 failed; 4 pre-existing dead-code warnings remain.
- Final Windows Release build succeeded at `target/release/aitext.exe`.
- Isolated Windows UI verification confirmed System/简体中文/English switching, White/Dark editor themes inside one shell, centered Settings with fixed Save/Close footer, vector close controls, Shift IME switching, caret-anchored IME candidates, Enter newline behavior, and no CMD child process.
- An isolated localhost mock endpoint produced visible green ghost text; no vendor endpoint or real API key was used.
- Independent Impeccable review first returned `fix` for Settings geometry and close controls; the bounded fix pass scored both resolved and returned `ship`.
- Final design documentation was regenerated from the shipped code into canonical `DESIGN.md` plus `.impeccable/design.json`.
- Both isolated Aitext processes, both exact temporary config directories, and the local mock server were stopped/removed. The obsolete first review capture was deleted; the two final review captures remain.

## 2026-08-20 — Compact neutral editor gutter

- User approved a bounded layout refinement: remove the orange manuscript rule, shrink the line-number gutter, and align the initial caret close to a neutral divider.
- TDD RED captured the old 50 px divider position; GREEN now renders a 34 px gutter, right-aligned line numbers 6 px before the divider, and text/caret 4 px after it.
- Divider color is derived as a translucent neutral gray from each theme's gutter tone; orange manuscript tokens were removed from runtime and design documentation.
- Final workspace suite: 164 tests passed, 0 failed; Release build succeeded with the same 4 existing dead-code warnings.
- Computer Use visual inspection was stopped immediately when the user pressed physical Escape. Per user instruction, future builds will be handed off for manual testing without desktop control.
- The isolated visual-test process and temp configuration were removed. Two obsolete review screenshots showing the old orange line were deleted and are not recoverable; they were generated test artifacts only.

## 2026-08-20 — Input isolation, light shell, and visible sliders

- Root cause of duplicate Profile/document input: the editor cloned global `Event::Text` before the Settings text field consumed it. Editor key/text handling is now disabled while Settings or Find owns input.
- Root cause of black White-theme chrome: all ordinary themes shared the dark `ShellColors`. White and macOS Light now use a dedicated light shell family; dark themes retain the dark shell.
- Root cause of invisible Slider tracks: the inactive rail matched the Settings background and trailing fill was disabled. All Settings sliders now use a contrasting rule-colored track and blue value fill.
- Added three focused regressions covering Settings input isolation, light-theme shell tokens, and visible slider track/value fill.
- Final workspace suite: 167 tests passed, 0 failed. Release build succeeded with the same 4 existing dead-code warnings.
- Per user instruction, no app or desktop automation was used; this build is handed off for manual visual testing.

## 2026-08-20 — Aitext v0.1.0 formally published

- Windows RC workflow `32371624434` passed before the repository restart; the hosted installer and portable archive were downloaded, checked, and not launched.
- Public repository is now [naipi11/Aitext](https://github.com/naipi11/Aitext). Old Helix Copilot releases/tags were removed, `main` was replaced with the reviewed Aitext commit, and only `v0.1.0` remains.
- Formal tag workflow `32372786776` passed all formatting/tests/release-build/package steps and published the non-draft [Aitext v0.1.0 Release](https://github.com/naipi11/Aitext/releases/tag/v0.1.0).
- Final release assets were downloaded and verified against `SHA256SUMS.txt`; ZIP contents were inspected; `Get-AuthenticodeSignature` reported `NotSigned`; the installer was not run.
- Temporary RC branch `codex/aitext-v0.1.0-rc` was deleted after final Release verification. No desktop automation was used.

## 2026-08-20 — README showcase readability revision

- Replaced both Paper Cut hero images with readable editor examples: a natural-language ghost continuation plus a Python `print("Hello, World!")` completion.
- Updated both English and Simplified Chinese GIFs to use the same visible examples with a blinking caret, rather than abstract placeholder bars.
- Verified PNG dimensions (1672×941), GIF dimensions (1200×675), six GIF frames at 5 fps, and clean `git diff --check`. No application or desktop automation was used.

## 2026-08-20 — Aitext v0.1.0 formal release planning

- User requested Windows installer, repository rename to Aitext, formal GitHub Release, bilingual Quick Start README, and English/Chinese showcase PNG/GIF assets.
- User selected destructive restart strategy B: remove old Helix Copilot tags/releases and restart at v0.1.0 without a legacy branch.
- User selected unsigned installer strategy A; no signing certificate secrets are present.
- User selected Paper Cut for the bilingual visual system.
- Read-only GitHub and packaging checks completed; no external mutation, commit, push, rename, tag deletion, or release creation has occurred.
