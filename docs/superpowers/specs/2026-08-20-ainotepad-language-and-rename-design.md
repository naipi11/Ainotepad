# Ainotepad language, typography, and product replacement design

Date: 2026-08-20  
Status: ready for user review  
Scope: replace the current Aitext v0.1.0 product identity with Ainotepad, add a selectable document type model with syntax highlighting, and correct mixed Chinese/Latin editor layout.

## 1. Outcome

Ainotepad is the direct replacement for the current Aitext v0.1.0 product. It remains a lightweight Windows-native notepad with inline ghost text, but its editor now treats document type as an explicit user-facing state rather than an inferred status label.

The release has three visible outcomes:

1. Chinese, Latin, punctuation, line numbers, caret, IME preedit, selection, and ghost text share one measured line layout and no longer drift vertically or horizontally.
2. The status bar exposes a compact document-type selector. New untitled documents default to Markdown; opened files use extension detection and can be overridden manually.
3. Every public product surface says Ainotepad: executable, window title, installer, README files, showcase media, configuration namespace, repository name, and the replacement v0.1.0 Release.

The numeric release version remains `0.1.0`. The product name changes; the version is not bumped to `0.1.1` because the user requested a direct replacement of the current release.

## 2. Product boundary

Included:

- Unified mixed-script editor layout with measured caret and ghost positions.
- A language/document-type selector in the existing compact status rail.
- Markdown as the default language for new untitled documents.
- Automatic extension detection for common text and programming formats.
- Syntax highlighting for Markdown, plain text, C, C++, C#, Python, Rust, JavaScript, TypeScript, HTML, CSS, JSON, TOML, PowerShell, Batch, and INI.
- Localized English and Simplified Chinese labels for the document-type selector.
- Full Aitext → Ainotepad product and repository replacement while retaining numeric version `0.1.0`.
- Safe migration of legacy `%APPDATA%\Aitext` or `%LOCALAPPDATA%\Aitext` configuration directories to `%LOCALAPPDATA%\Ainotepad` without logging or exposing API keys.
- Updated bilingual README, Quick Start, release notes, installer, CI/release workflows, and English/Chinese showcase PNG/GIF assets.

Not included:

- LSP, diagnostics, code completion lists, folding, minimap, project tree, terminal, plugins, or a permanent IDE sidebar.
- Runtime translation downloads or web UI dependencies.
- Provider protocol changes unrelated to this product rename.
- Automatic deletion of the legacy Aitext configuration directory.

## 3. Editor typography and mixed-script layout

### 3.1 Root cause

The current painter lays out and paints each scalar value as an independent one-character galley. It then estimates some fallback widths with a fixed `M`-based heuristic. Chinese glyphs can come from a different fallback face than Latin glyphs, so their baseline, ascent, and advance are not guaranteed to share the same line metrics. The ghost string is painted separately at the caret and can therefore appear visually detached from the shaped document text.

### 3.2 Layout contract

Each visible document line is shaped as one `egui::LayoutJob` using the selected editor family and its installed fallback chain. Syntax tokens contribute text sections and colors to that job; they do not create independent vertical layout contexts.

The line layout object owns:

- The line `Galley` used for painting.
- A mapping from document scalar offsets to x positions.
- The line height and baseline used by the current-line fill, caret, IME rectangle, selection, and ghost preview.
- A reverse mapping used by mouse hit testing.

The same layout object is used for:

- Normal document text.
- Mixed Chinese/Latin/punctuation sequences.
- The current caret.
- IME preedit text.
- Selection rectangles.
- Ghost text positioned after the exact shaped prefix.

The fixed fallback-width approximation is removed from normal layout. Tabs use the configured tab width through the line layout; a missing glyph falls back through the registered font family without inventing a second advance rule.

### 3.3 Font policy

The current user-selected editor family remains selectable. Ainotepad registers the available Windows faces in a deterministic fallback order:

- `YaHei` / Microsoft YaHei for CJK coverage.
- `SimHei` as a CJK fallback.
- `Consolas` and `Cascadia Mono` for code glyph coverage.
- `Segoe UI` for general Latin fallback.

The selected family is first in the chain, but every editor line can resolve CJK and Latin glyphs through the same `FontId`/fallback configuration. Chrome typography remains separate from document typography.

### 3.4 Regression case

The painter test suite includes a mixed-script line equivalent to:

```text
你好###abccABCDA你好###
```

The test checks that the line galley is used for both the text and caret mapping, that the caret advances monotonically across CJK and Latin spans, and that the ghost origin equals the shaped prefix end rather than a fixed character-count estimate.

## 4. Document type model

### 4.1 Language identifiers

`LanguageId` becomes the single source of truth for extension detection, manual selection, status-bar labels, lexer routing, and completion context:

```rust
pub enum LanguageId {
    Markdown,
    PlainText,
    C,
    Cpp,
    CSharp,
    Python,
    Rust,
    JavaScript,
    TypeScript,
    Html,
    Css,
    Json,
    Toml,
    PowerShell,
    Batch,
    Ini,
}
```

`Document::from_text` initializes a new untitled document as `Markdown`. A recognized plain-text extension such as `.txt` or `.text` resolves to `PlainText`. An unknown extension and an extensionless opened file resolve to `Markdown` unless the user manually changes the type.

### 4.2 Extension mapping

| Type | Extensions |
| --- | --- |
| Markdown | `.md`, `.markdown`, `.mdown` |
| Plain Text | `.txt`, `.text`, `.log` |
| C | `.c`, `.h` |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx` |
| C# | `.cs` |
| Python | `.py`, `.pyw` |
| Rust | `.rs` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| TypeScript | `.ts`, `.tsx` |
| HTML | `.html`, `.htm` |
| CSS | `.css` |
| JSON | `.json`, `.jsonc` |
| TOML | `.toml` |
| PowerShell | `.ps1`, `.psm1`, `.psd1` |
| Batch | `.bat`, `.cmd` |
| INI | `.ini`, `.cfg`, `.conf` |

The mapping remains case-insensitive. Manual selection changes the current document only and never changes its file contents or extension.

### 4.3 Status-bar selector

The existing `Language` status item becomes an interactive compact control. It keeps the current Paper Cut status-rail geometry and uses the same theme-aware hover/focus treatment as other controls.

```text
UTF-8   LF   [ Markdown ▾ ]   Imported DeepSeek · model   suggested
```

The popup is grouped into two readable sections:

```text
Text
  Markdown
  Plain Text

Programming
  C       C++       C#       Python
  Rust    JavaScript       TypeScript
  HTML    CSS       JSON     TOML
  PowerShell         Batch   INI
```

The selector is available even when the status-bar language item is hidden: the editor context surface exposes the same `Document type` action through the Edit menu. This keeps the setting discoverable without adding a permanent toolbar.

Changing the type immediately re-highlights the document and updates the completion language context. It does not change text, caret, encoding, newline style, profile, API key, or undo history.

## 5. Syntax highlighting

### 5.1 Lexer architecture

The existing `TokenKind` and `highlight(text, language)` interfaces remain. New lexers are small, deterministic, allocation-bounded scanners in `aitext-core/src/lexers`:

- `csharp.rs`: C# keywords, types, attributes, strings, comments, numbers, and common control words.
- `html.rs`: tags, attributes, quoted attribute values, comments, entities, and text.
- `css.rs`: selectors, properties, values, numbers, colors, strings, comments, and punctuation.

C and C++ continue to share the C-family lexer with language-specific keyword tables. Existing Markdown, Python, Rust, JavaScript/TypeScript, JSON, TOML, PowerShell, Batch, and INI lexers remain compatible and receive extension coverage where required.

### 5.2 Highlighting contract

- All tokens cover the complete document through the existing gap-filling pass.
- Unterminated strings/comments remain safe and render the remainder as the corresponding token kind.
- Unknown syntax falls back to `Text` without panicking or blocking editing.
- Language switching re-runs highlighting on the next frame and never mutates document text.
- Syntax colors are provided by the active editor theme; the language selector does not hard-code colors.

## 6. Ainotepad product replacement

### 6.1 Public and internal identity

The visible product and Cargo package identity become Ainotepad:

- Product name: `Ainotepad`.
- Binary: `ainotepad.exe`.
- Application crate: `ainotepad`.
- Core crate: `ainotepad-core`.
- AI crate: `ainotepad-ai`.
- Rust types: `AinotepadApp` and related user-facing symbols.
- Windows metadata, window title, icon resource names, installer text, and output names.

The numeric workspace version remains `0.1.0`.

### 6.2 Configuration migration

On startup:

1. Prefer `%LOCALAPPDATA%\Ainotepad`.
2. If it does not exist and either `%APPDATA%\Aitext` or `%LOCALAPPDATA%\Aitext` exists, copy the legacy configuration and DPAPI secret files into the Ainotepad directory while preserving file contents and permissions as far as Windows permits.
3. Load and validate the Ainotepad configuration.
4. Keep the old Aitext directory as a recoverable backup; never log or display secret contents.

If migration fails, Ainotepad starts with defaults and reports a concise recoverable status message. The application must remain usable with no configuration directory, a malformed legacy config, or an unreadable legacy DPAPI file.

### 6.3 Documentation and media

Replace Aitext branding in:

- `README.md` and `README.zh-CN.md`.
- Quick Start, download links, badges, release notes, PRODUCT, DESIGN, and design metadata.
- `docs/media/hero-en.png`, `hero-zh-CN.png`, `demo-en.gif`, and `demo-zh-CN.gif`.
- Media provenance documentation and image-generation prompts.
- Packaging and workflow filenames, descriptions, and artifact names.

Showcase media must visibly say Ainotepad and demonstrate both a natural-language ghost suggestion and a code completion. The English and Chinese variants use matching layout, exact localized copy, unified editor typography, and character-by-character suggestion/code animation.

## 7. Public repository and release replacement

After local and hosted verification:

1. Push a temporary `codex/ainotepad-v0.1.0-rc` branch and run CI/release packaging without launching the installer.
2. Rename `naipi11/Aitext` to `naipi11/Ainotepad` and update the repository description/topics.
3. Update the local `origin` URL and replace `main` with the reviewed Ainotepad commit.
4. Delete the existing Aitext `v0.1.0` Release and tag only after RC artifacts are verified.
5. Create and push the replacement `v0.1.0` tag.
6. Publish the non-draft `Ainotepad v0.1.0` Release with:
   - `Ainotepad-Setup-0.1.0-win-x64.exe`
   - `Ainotepad-Portable-0.1.0-win-x64.zip`
   - `SHA256SUMS.txt`
7. Verify the published assets and hashes, then delete the temporary RC branch.

The installer remains unsigned under the previously approved release policy. No installer is launched by the agent; Windows installation and manual UI verification remain user-owned.

## 8. Error handling and compatibility

- Unknown document extensions fall back to Markdown.
- Unknown serialized language values fall back to Markdown for documents and preserve unrelated configuration fields.
- A lexer failure falls back to plain text tokens and never blocks input.
- A type change clears no completion text by itself unless the completion language context changes; any stale request is invalidated through the existing completion generation mechanism.
- Rename/migration failures never remove the old Aitext directory or API secrets.
- All user-facing errors are localized through the existing typed catalog.

## 9. Test strategy

Before implementation, add failing tests for:

1. New untitled documents default to `Markdown`.
2. `.txt`/`.text` resolve to `PlainText`; unknown and extensionless documents resolve to `Markdown`.
3. C, C++, C#, HTML, CSS, Python, Markdown, and the other listed extensions resolve deterministically, case-insensitively.
4. C# keywords/types, HTML tags/attributes, and CSS properties/colors produce expected token kinds.
5. Every lexer output covers the complete source without gaps or overlaps.
6. Manual language selection changes only the current document language and invalidates the stale completion context.
7. A mixed Chinese/Latin line uses one line galley for painting, caret positioning, selection, IME, and ghost origin.
8. Existing settings, provider profiles, theme values, and status-bar settings survive the Ainotepad config migration without plaintext API keys.
9. A malformed or missing legacy Aitext config does not prevent Ainotepad startup.
10. Product metadata, installer names, repository links, and README download links contain Ainotepad and retain numeric version `0.1.0`.

No test launches a vendor API, contains a real credential, starts the desktop app, or runs the installer.

## 10. Acceptance criteria

- The mixed text from the reported screenshot renders with a shared baseline and correct caret/ghost alignment.
- New documents show `Markdown` in the type selector and syntax-highlight Markdown immediately.
- The selector supports all listed mainstream text/programming formats and updates highlighting without editing content.
- C, C++, C#, Python, HTML, CSS, Markdown, and plain text examples visibly highlight correctly.
- Ainotepad starts with no Aitext branding in the window, installer, README, status surfaces, or showcase media.
- Existing Aitext configuration can migrate without exposing or losing API keys.
- The public repository is `naipi11/Ainotepad` and the replacement `Ainotepad v0.1.0` Release contains verified unsigned Windows installer and portable ZIP assets.
- Full workspace tests, formatting, release build, hosted packaging, and SHA256 verification pass.
- No desktop automation is used; the user receives the final build for manual Windows testing.
