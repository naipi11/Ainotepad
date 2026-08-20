# Aitext Paper Cut UI and bilingual interface design

Date: 2026-08-20
Status: ready for user review
Scope: replace the current generic application chrome with the approved Paper Cut visual system and add a complete System / Simplified Chinese / English interface language setting.

## 1. Outcome

Aitext should look like the approved README hero rather than a collection of default egui controls. Its stable identity is a precise native Windows shell around a writing surface, with light chrome for light themes, dark chrome for dark themes, a compact neutral-gray gutter divider, restrained blue focus accents, and green ghost text.

The interface must also stop mixing Chinese and English. Every user-facing string is selected from one locale at a time. The user can choose:

- Follow Windows / 跟随 Windows
- 简体中文
- English

The selected language applies immediately and persists through `config.toml` when settings are saved.

## 2. Product boundary

Included:

- Application shell, menu rail, tabs, status bar, editor framing, Settings, Find/Replace, About, and shortcut help styling.
- Stable Paper Cut component tokens shared by every theme.
- Theme-aware editor surfaces inside the stable shell.
- Complete Simplified Chinese and English UI catalogs.
- Windows system-locale resolution for the System setting.
- Configuration migration, unit tests, release build, and bounded Windows visual verification.

Not included:

- File tree, activity bar, command palette, LSP, terminal, plugin system, WebView, or chat sidebar.
- Custom title-bar replacement or removal of normal Windows minimize, maximize, and close behavior.
- Additional languages beyond Simplified Chinese and English.
- Runtime translation downloads or external localization services.

## 3. Visual world: Paper Cut

### 3.1 Core idea

The interface combines two familiar writing materials:

- A compact line-number gutter and one quiet neutral-gray divider.
- A precise charcoal-black editor instrument with restrained blue controls.

Green belongs exclusively to ghost completion and completion-ready states. This preserves immediate semantic recognition: blue means navigation/focus; green means suggested text; the gray gutter divider carries no state.

### 3.2 Stable shell tokens

The application structure remains stable across themes. White and macOS Light use light shell colors; dark themes use the dark shell palette.

| Role | Token | Intent |
|---|---|---|
| Shell | `#101113` | Matte native frame and menu rail |
| Raised shell | `#17191D` | Menus, Settings, transient surfaces |
| Shell hover | `#232730` | Hover and selected navigation state |
| Shell text | `#E7E9ED` | Primary chrome text |
| Muted shell text | `#9BA1AA` | Secondary labels and inactive status |
| Focus blue | `#3B82F6` | Current tab, focused section, keyboard focus |
| Ghost green | `#5BD68B` | Ghost text and completion-ready indication |
| Editor divider | translucent neutral gray | Single thin gutter separator only |
| Shell rule | `#30343B` | Hairline separation |

No gradients, glass blur, decorative neon, thick colored borders, or card stacks are introduced.

### 3.3 Application structure

```text
┌ Windows title bar ───────────────────────────────────────────────┐
├ menu rail ───────────────────────────── provider / completion ──┤
├ tabs ── active tab blue rule ───────────────────────────────────┤
│ line numbers │ gray divider │ editor surface                   │
│              │             │ text  green ghost continuation    │
│              │             │                                   │
├ status rail ─ encoding · language · profile · completion ──────┤
└─────────────────────────────────────────────────────────────────┘
```

The editor retains maximum usable area. There is no permanent file sidebar or decorative activity rail.

### 3.4 Components

- **Menu rail:** 34–36 px neutral strip. Menus use one locale, compact spacing, theme-family raised surfaces, and a blue focus/hover indication.
- **Tabs:** thin dark rail. The active tab uses a 2 px blue bottom rule and a slightly raised fill. Dirty state remains `*` until a dedicated icon system exists.
- **Editor:** retains compact right-aligned line numbers, caret, syntax colors, selection, IME, and a neutral-gray gutter divider. The current line uses a quiet theme-relative fill.
- **Ghost text:** uses theme-adjusted green with sufficient contrast and no glow inside the real application.
- **Status bar:** dark stable rail with short separators and completion state at the right side when space permits.
- **Settings:** centered dark tool window with the existing AI Profiles / Appearance / Status Bar sections, compact controls, fixed Save and Close actions, and visible keyboard focus.
- **Find/Replace:** dark inline bar, not a floating card. Field/action order follows the current editing flow.
- **About and shortcut help:** use the same dark tool-surface language and localized text.

### 3.5 Theme interaction

White, Black Green, VS Code Dark, macOS Light, Dark, Lamp paper, High contrast, and Custom continue to control the editor page, syntax palette, selection, current line, and ghost contrast.

The stable structure does not become a different product for every theme. White and macOS Light use crisp light chrome; dark themes use dark chrome while preserving the same boundaries and hierarchy. High Contrast may override shell colors when accessibility requires it. Custom continues to control user-authored editor colors while inheriting the Paper Cut structure.

## 4. Interface-language architecture

### 4.1 Configuration

Add a serialized setting:

```rust
pub enum UiLanguage {
    System,
    ZhCn,
    En,
}

AppConfig {
    ui_language: UiLanguage,
    // existing settings
}
```

`System` is the default for new and existing configurations. Missing or unknown future values deserialize to `System`; no existing configuration becomes invalid and an unsupported language value does not reset unrelated settings.

### 4.2 Locale resolution

`UiLanguage::System` resolves the Windows user locale through the Win32 locale API already available through the Windows dependency. Locales beginning with `zh` resolve to `ZhCn`; every other locale resolves to English. Locale detection is isolated behind an injectable resolver so tests do not depend on the developer machine.

### 4.3 Translation catalog

Create `i18n.rs` with a typed key rather than scattered string lookups:

```rust
pub enum TextKey {
    MenuFile,
    FileNew,
    FileOpen,
    // every static interface string
}

pub fn text(locale: Locale, key: TextKey) -> &'static str;
```

Benefits:

- Both locales must handle every key at compile time.
- No TOML/JSON translation files or runtime parsing are needed.
- Static labels return `&'static str` without heap allocation.
- Missing translation behavior cannot silently produce a mixed-language interface.

Formatted messages use typed helper functions with locale-specific complete sentence templates. Dynamic errors keep technical details but localize the surrounding explanation and recovery action.

### 4.4 Coverage

The catalog covers:

- File, Edit, Find, Settings, and Help menus.
- Tab and empty-editor states.
- Find/Replace controls.
- Settings headings, sections, labels, buttons, helper text, confirmations, and status messages.
- Provider labels and adapter labels where they are product terms; API model IDs and URLs remain unchanged.
- Status-bar labels and completion states.
- File/open/save/encoding errors.
- About and keyboard-shortcut help.

Brand names, model IDs, encodings, API protocol names, URLs, and keyboard chords are not translated.

### 4.5 Language selector

The selector appears at the top of Appearance under Interface language / 界面语言. Options remain self-identifying even when the current locale is unfamiliar:

- 跟随 Windows / Follow Windows
- 简体中文
- English

Changing it updates the in-memory locale immediately, invalidates layout for the next frame, and leaves document content, cursor, IME state, completion generation, profiles, and API keys untouched. Save settings persists the choice.

## 5. Data flow

```text
saved UiLanguage
       │
       ├─ System ─ Win32 locale resolver ─┐
       ├─ ZhCn ────────────────────────────┤
       └─ En ──────────────────────────────┤
                                           ▼
                                    resolved Locale
                                           │
                     ┌─────────────────────┼──────────────────────┐
                     ▼                     ▼                      ▼
                 menu/app              Settings              messages
```

The resolved locale is cheap to recompute and may be stored in `AitextApp` if needed. Translation never runs in completion worker threads and never changes request payloads.

## 6. Error handling and security

- Language switching does not read, rewrite, log, or display API keys.
- Failed system-locale detection falls back to English.
- Unknown future serialized language values resolve to System without discarding unrelated configuration fields.
- Provider/network error details remain sanitized by the existing key-redaction path before localization.
- The redesign does not introduce telemetry, network assets, web fonts, or external UI dependencies.

## 7. Accessibility and interaction

- Windows title controls and resize behavior remain native.
- Keyboard focus is visible in blue and is not communicated by color alone where text selection already exists.
- Menu and Settings order match keyboard traversal order.
- Simplified Chinese uses YaHei first; English chrome uses Segoe UI where available; editor text keeps the configured monospaced/CJK fallback chain.
- Controls allow Chinese expansion without clipping at the current minimum Settings size.
- High Contrast remains an explicit theme and may override decorative Paper Cut tokens.
- Enter, Tab, Esc, IME composition, and ghost acceptance semantics remain unchanged.

## 8. Test strategy

Automated tests must cover:

1. Missing language configuration defaults to System.
2. Injected `zh-CN`, `zh-TW`, and non-Chinese Windows locales resolve deterministically.
3. Every `TextKey` returns non-empty Simplified Chinese and English text.
4. Representative menu, Settings, Find/Replace, status, error, and confirmation keys differ appropriately between locales.
5. Product terms such as FIM, Chat Completions, Responses API, model IDs, and keyboard chords remain stable.
6. Switching locale does not modify documents, profiles, secrets, completion generation, or IME state.
7. Existing theme and provider configuration round trips with the new setting.
8. Paper Cut shell tokens meet representative contrast checks.
9. Existing editor, completion, provider, secret, theme, and settings tests continue to pass.

Windows visual verification uses isolated configuration and checks:

- White editor page inside the light Paper Cut shell.
- A dark editor theme inside the same shell.
- English menu/Settings with no Chinese labels.
- Simplified Chinese menu/Settings with no unintended English prose.
- Immediate language switching, Settings close methods, editor typing, and no command window.

## 9. Delivery order

1. `UiLanguage`, locale resolver, typed translation keys, and catalog tests.
2. Menu, Find/Replace, status, dialogs, errors, and Settings localization.
3. Paper Cut semantic shell tokens and application chrome.
4. Settings language selector and immediate switch behavior.
5. Full workspace tests, Release build, and bounded Windows visual verification.

## 10. Acceptance criteria

- The application visibly matches the approved Paper Cut direction: matte-black shell, blue focus, compact neutral-gray gutter divider, and green ghost text.
- No permanent IDE sidebar or unrelated feature is added.
- System, 简体中文, and English options are visible and persist correctly.
- English mode contains no unintended Chinese interface prose except the self-identifying `简体中文` language option.
- Simplified Chinese mode contains no unintended English interface prose except the self-identifying `English` language option, product/API terms, model IDs, URLs, encodings, and shortcuts.
- Language changes apply without restarting and without altering documents or AI configuration.
- Existing themes remain selectable and coherent inside the stable shell.
- Chinese IME, caret placement, Enter, Tab, Esc, ghost invalidation, menus, Settings close behavior, and no-console startup remain intact.
- Full workspace tests and the Windows Release build pass without live vendor credentials.
