# Ainotepad v0.1.0 Replacement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Replace the current Aitext v0.1.0 product with Ainotepad, add a Markdown-default document-type selector and mainstream syntax highlighting, and eliminate mixed Chinese/Latin editor alignment drift.

**Architecture:** Keep the existing Rust workspace boundaries while renaming the public/internal packages to ainotepad, ainotepad-core, and ainotepad-ai. Add a typed LanguageId model in the core, route it through extension detection, lexers, status-bar selection, and completion context, and render each editor line through one egui LayoutJob/Galley so all glyphs share measured metrics. Finish with a hosted Windows RC proof, then replace the public Aitext repository and v0.1.0 Release with Ainotepad v0.1.0.

**Tech Stack:** Rust stable, Cargo workspace, eframe/egui 0.31, ropey, deterministic lexers, Windows DPAPI/config migration, Inno Setup, GitHub Actions, GitHub CLI, ffmpeg, built-in image generation, Markdown.

**Spec:** docs/superpowers/specs/2026-08-20-ainotepad-language-and-rename-design.md

## Global Constraints

- Keep numeric version 0.1.0; replace the product name and public assets with Ainotepad.
- New untitled documents default to Markdown; .txt, .text, and .log resolve to Plain Text.
- Support C, C++, C#, Python, Rust, JavaScript, TypeScript, HTML, CSS, Markdown, Plain Text, JSON, TOML, PowerShell, Batch, and INI.
- Use one line-level egui layout for mixed Chinese/Latin text, caret, IME preedit, selection, and ghost text.
- Preserve the Paper Cut shell, themes, provider profiles, DPAPI protection, IME behavior, shortcuts, and lightweight no-sidebar boundary.
- Migrate %LOCALAPPDATA%\Aitext to %LOCALAPPDATA%\Ainotepad without logging secrets or automatically deleting the legacy directory.
- Do not make live vendor API requests, launch the desktop app, launch the installer, or use desktop automation.
- Do not use git reset --hard, git checkout --, git add -A, or git add ..
- Stage only explicitly reviewed paths and preserve unrelated user changes.
- Perform public rename, old Release/tag deletion, force-update of main, and replacement v0.1.0 publication only after local and hosted RC evidence is green.

## File map

| Unit | Files | Responsibility |
| --- | --- | --- |
| Core document type | crates/aitext-core/src/language.rs, document.rs, workspace.rs | Language IDs, defaults, extension mapping, per-document selection |
| Core highlighting | crates/aitext-core/src/highlight.rs, src/lexers/* | Token routing and deterministic scanners |
| Editor layout | crates/aitext/src/line_layout.rs, painter.rs, editor_view.rs | One shaped line and shared cursor metrics |
| Type selector | crates/aitext/src/status_bar.rs, commands.rs, i18n/* | Interactive status-bar language control |
| Brand/config | Cargo manifests, config.rs, main.rs, assets | Package identity, executable identity, config migration |
| Documentation/media | README files, PRODUCT.md, DESIGN.md, docs/media/* | Public Ainotepad messaging and showcase |
| Packaging/release | packaging/windows/*, .github/workflows/* | Installer, portable ZIP, CI, replacement Release |

---

### Task 1: Add the document-type model and Markdown default

**Files:**
- Modify: crates/aitext-core/src/language.rs
- Modify: crates/aitext-core/src/document.rs
- Modify: crates/aitext-core/src/workspace.rs
- Test: the same modules

**Interfaces:**
- Add CSharp, Html, and Css to LanguageId.
- Add LanguageId::ALL in the stable selector order.
- Keep language_from_path(path: &str) -> LanguageId case-insensitive.
- Make Document::from_text initialize Markdown.
- Keep Document::set_path extension detection for real paths.

- [ ] Step 1: Write failing tests

~~~rust
#[test]
fn new_document_defaults_to_markdown() {
    assert_eq!(Document::new().language(), LanguageId::Markdown);
}

#[test]
fn mainstream_extensions_are_case_insensitive() {
    assert_eq!(language_from_path("main.CPP"), LanguageId::Cpp);
    assert_eq!(language_from_path("Program.Cs"), LanguageId::CSharp);
    assert_eq!(language_from_path("page.HTML"), LanguageId::Html);
    assert_eq!(language_from_path("theme.CSS"), LanguageId::Css);
    assert_eq!(language_from_path("note.unknown"), LanguageId::Markdown);
}
~~~

- [ ] Step 2: Run RED

Run cargo test -p aitext-core language::tests::new_document_defaults_to_markdown -- --exact and cargo test -p aitext-core language::tests::mainstream_extensions_are_case_insensitive -- --exact. Expected: failure because the default and new variants are absent.

- [ ] Step 3: Implement the minimal model

Add the variants, complete extension table, and Markdown initializer. Cover .md, .markdown, .txt, .text, .log, .c, .h, .cpp, .cc, .cxx, .cs, .py, .rs, .js, .ts, .tsx, .html, .htm, .css, .json, .jsonc, .toml, .ps1, .bat, .cmd, .ini, .cfg, and .conf.

- [ ] Step 4: Run GREEN and commit

Run cargo test -p aitext-core language document workspace -- --test-threads=1, then:

~~~text
git add -- crates/aitext-core/src/language.rs crates/aitext-core/src/document.rs crates/aitext-core/src/workspace.rs
git commit -m "feat: add markdown-default document types"
~~~

### Task 2: Add C#, HTML, and CSS highlighting

**Files:**
- Create: crates/aitext-core/src/lexers/csharp.rs
- Create: crates/aitext-core/src/lexers/html.rs
- Create: crates/aitext-core/src/lexers/css.rs
- Modify: crates/aitext-core/src/lexers/mod.rs and highlight.rs
- Test: lexer and highlighter modules

**Interfaces:**
- Each lexer exposes pub fn lex(text: &str) -> Vec<Token>.
- highlight(text, LanguageId) routes the new variants.
- Returned ranges are ordered and half-open; fill_gaps covers untouched text.

- [ ] Step 1: Write failing tests

~~~rust
#[test]
fn csharp_highlights_type_method_and_string() {
    let tokens = highlight("public class Demo { string name = \"A\"; }", LanguageId::CSharp);
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Type));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
}

#[test]
fn html_highlights_tag_attribute_and_comment() {
    let tokens = highlight("<!-- x --><div class=\"app\">Hi</div>", LanguageId::Html);
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Comment));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
}

#[test]
fn css_highlights_selector_property_and_number() {
    let tokens = highlight(".app { color: #fff; margin: 4px; }", LanguageId::Css);
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Number));
}
~~~

- [ ] Step 2: Run RED

Run the three focused tests separately: cargo test -p aitext-core highlight::tests::csharp_highlights_type_method_and_string -- --exact, cargo test -p aitext-core highlight::tests::html_highlights_tag_attribute_and_comment -- --exact, and cargo test -p aitext-core highlight::tests::css_highlights_selector_property_and_number -- --exact. Expected: failure because the new language routes and scanners do not exist.

- [ ] Step 3: Implement scanners

Follow the existing lexer style. Recognize C# keywords, types, attributes, strings, and comments; HTML tags, attributes, quoted values, comments, and entities; CSS selectors, properties, colors, strings, comments, and numbers. Unterminated constructs must return safely.

- [ ] Step 4: Add coverage tests

For each new lexer, assert the final token list covers the whole input with no gaps or overlaps and add malformed string/comment/tag cases.

- [ ] Step 5: Run GREEN and commit

Run cargo test -p aitext-core highlight lexers language -- --test-threads=1, then:

~~~text
git add -- crates/aitext-core/src/highlight.rs crates/aitext-core/src/lexers
git commit -m "feat: highlight csharp html and css"
~~~

### Task 3: Replace per-character painting with shared line layout

**Files:**
- Create: crates/aitext/src/line_layout.rs
- Modify: crates/aitext/src/lib.rs and painter.rs
- Test: line_layout.rs and painter.rs

**Interfaces:**
- EditorLineLayout stores the line scalar range and an Arc<egui::Galley>.
- EditorLineLayout::x_for_offset(offset: usize) -> f32 uses the shaped galley row.
- EditorLineLayout::offset_at_x(x: f32) -> usize uses the same row hit test.
- build_line_layout(ui, font, line_text, token_spans, colors) -> EditorLineLayout builds one LayoutJob.
- paint_editor uses the object for painting, selection, caret, IME, mouse hit testing, and ghost origin.

- [ ] Step 1: Write failing mixed-script tests

Use the reported text:

~~~rust
#[test]
fn mixed_script_layout_uses_one_galley() {
    let layout = build_test_layout("你好###abccABCDA你好###");
    assert_eq!(layout.galley.job.text, "你好###abccABCDA你好###");
    assert!(layout.x_for_offset(6) > layout.x_for_offset(5));
    assert_eq!(layout.offset_at_x(layout.x_for_offset(5) + 0.1), 5);
}
~~~

Add an egui painter test that verifies the ghost origin equals the caret x on the mixed line and that the caret x is not reset to the line origin after CJK glyphs.

- [ ] Step 2: Run RED

Run cargo test -p aitext line_layout::tests::mixed_script_layout_uses_one_galley -- --exact and the painter regression. Expected: failure because the shared line helper does not exist.

- [ ] Step 3: Implement shaping

Build a LayoutJob with one TextFormat section per syntax span, shape through ui.fonts(|fonts| fonts.layout_job(job)), and use Galley::pos_from_ccursor, Galley::cursor_from_pos, and Row::x_offset rather than the current fixed M fallback heuristic.

- [ ] Step 4: Replace the painter loop

Paint one galley per line. Derive line height from the row. Use the same row for current-line fill, selection rectangles, caret, IME preedit, mouse hit testing, and ghost origin. Keep the 34 px gutter and neutral divider.

- [ ] Step 5: Run GREEN and the complete suite

Run:

~~~text
cargo test -p aitext painter editor_view line_layout -- --test-threads=1
cargo test --workspace -- --test-threads=1
~~~

- [ ] Step 6: Commit

~~~text
git add -- crates/aitext/src/lib.rs crates/aitext/src/line_layout.rs crates/aitext/src/painter.rs
git commit -m "fix: align mixed script editor layout"
~~~

### Task 4: Add the interactive document-type selector

**Files:**
- Modify: crates/aitext/src/status_bar.rs, commands.rs, i18n/message.rs, and i18n/catalog.rs
- Modify: crates/aitext/src/app.rs only if a shared status action helper is required
- Test: status bar, commands, and catalog modules

**Interfaces:**
- language_label(locale: Locale, language: LanguageId) -> &'static str.
- Command::SetDocumentLanguage(LanguageId).
- draw_language_selector(ui: &mut Ui, app: &mut AitextApp) before the package rename; the type is renamed to AinotepadApp in Task 5.
- Existing StatusItem::Language visibility remains respected; Edit exposes the same action if hidden.

- [ ] Step 1: Write failing tests

~~~rust
#[test]
fn language_labels_are_localized() {
    assert_eq!(language_label(Locale::En, LanguageId::Markdown), "Markdown");
    assert_eq!(language_label(Locale::ZhCn, LanguageId::PlainText), "纯文本");
    assert_eq!(language_label(Locale::En, LanguageId::CSharp), "C#");
}

#[test]
fn setting_document_language_preserves_text_and_caret() {
    let mut app = AitextApp::new_for_test();
    app.workspace.new_untitled();
    let doc = app.workspace.current_mut().unwrap();
    doc.insert("# 标题");
    let caret = doc.selection().caret;
    app.dispatch(Command::SetDocumentLanguage(LanguageId::Python));
    let doc = app.workspace.current().unwrap();
    assert_eq!(doc.language(), LanguageId::Python);
    assert_eq!(doc.text(), "# 标题");
    assert_eq!(doc.selection().caret, caret);
}
~~~

- [ ] Step 2: Run RED

Run the two focused tests. Expected: failure because localized labels and the command do not exist.

- [ ] Step 3: Add typed labels and command

Add English/Chinese labels for all listed types plus the Text/Programming group headings. The command changes only the current document language and increments the existing completion generation when the language context changes.

- [ ] Step 4: Build the compact popup

Render the selected language as a status-bar chip with grouped Text/Programming options, visible focus, outside-click close, and keyboard navigation. Selecting a type re-highlights immediately and preserves text, encoding, newline, selection, and provider state.

- [ ] Step 5: Run GREEN and commit

Run cargo test -p aitext status_bar commands i18n -- --test-threads=1 and the full workspace suite, then:

~~~text
git add -- crates/aitext/src/status_bar.rs crates/aitext/src/commands.rs crates/aitext/src/i18n
git commit -m "feat: add document type selector"
~~~

### Task 5: Rename packages, executable identity, and config namespace

**Files:**
- Rename: crates/aitext-core -> crates/ainotepad-core
- Rename: crates/aitext-ai -> crates/ainotepad-ai
- Rename: crates/aitext -> crates/ainotepad
- Rename: Aitext icon asset filenames to Ainotepad names
- Modify: workspace/manifests/lockfile, all Rust imports, user-facing symbols, config.rs, main.rs, and build.rs

**Interfaces:**
- cargo build --release -p ainotepad produces target/release/ainotepad.exe.
- Application type is AinotepadApp.
- config_dir() -> PathBuf returns %LOCALAPPDATA%\Ainotepad.
- legacy_config_dir() -> PathBuf returns %LOCALAPPDATA%\Aitext.
- migrate_legacy_config(new_dir: &Path, legacy_dir: &Path) -> io::Result<bool> copies the legacy directory only when the new directory is absent and never deletes the source.

- [ ] Step 1: Write failing migration tests

~~~rust
#[test]
fn legacy_config_is_copied_once_without_deleting_source() {
    let legacy = tempdir().unwrap();
    let new = tempdir().unwrap().path().join("Ainotepad");
    fs::write(legacy.path().join("config.toml"), "theme = \"white\"\n").unwrap();
    assert!(migrate_legacy_config(&new, legacy.path()).unwrap());
    assert!(new.join("config.toml").exists());
    assert!(legacy.path().join("config.toml").exists());
}
~~~

Also test that an existing Ainotepad directory wins and migration returns false.

- [ ] Step 2: Run RED

Run the focused config tests. Expected: failure because the new path and migration helper do not exist.

- [ ] Step 3: Implement migration and package rename

Use git mv for the three crate directories and icon assets. Update workspace members, package/dependency names, imports, binary metadata, include paths, test helpers, and Cargo.lock. Run migration before loading the new config; never print secret contents.

- [ ] Step 4: Update visible Windows identity

Change window title, eframe application name, AinotepadApp, icon include, OriginalFilename, FileDescription, ProductName, and config namespace. Keep Aitext only in compatibility migration code.

- [ ] Step 5: Run GREEN and commit

Run cargo fmt --all -- --check, cargo test --workspace -- --test-threads=1, and cargo build --release -p ainotepad. Then:

~~~text
git add -- Cargo.toml Cargo.lock crates/ainotepad-core crates/ainotepad-ai crates/ainotepad
git commit -m "refactor: replace Aitext identity with Ainotepad"
~~~

### Task 6: Update documentation and showcase media

**Files:**
- Modify: README.md, README.zh-CN.md, PRODUCT.md, DESIGN.md, and .impeccable/design.json
- Modify: docs/media/README.md and release-notes/v0.1.0.md
- Replace: docs/media/hero-en.png, hero-zh-CN.png, demo-en.gif, demo-zh-CN.gif
- Modify: task_plan.md, findings.md, and progress.md with evidence

**Interfaces:**
- README language switches and section order remain paired.
- Download links target https://github.com/naipi11/Ainotepad/releases/latest/download/.
- Showcase media visibly says Ainotepad and demonstrates sentence plus code completion.

- [ ] Step 1: Write the content audit

Use a PowerShell audit that fails if current public docs contain github.com/naipi11/Aitext, Aitext-Setup, Aitext-Portable, or Aitext v0.1.0 outside historical spec files, and asserts both READMEs contain Ainotepad, Quick Start, the language switch, and new asset names.

- [ ] Step 2: Run RED

Run rg -n "github.com/naipi11/Aitext|Aitext-Setup|Aitext-Portable|Aitext v0.1.0" README.md README.zh-CN.md PRODUCT.md DESIGN.md packaging .github release-notes. Expected: current Aitext branding is found.

- [ ] Step 3: Replace copy and links

Update English/Chinese Quick Start, installer names, package commands, repository links, release notes, product/design descriptions, and media provenance. Keep historical design/spec documents as historical records unless they are current user-facing documentation.

- [ ] Step 4: Generate and inspect matching media

Use built-in imagegen for the Paper Cut base and ffmpeg for exact text. Each GIF must show typed sentence, character-by-character green sentence ghost, typed print( prefix, and character-by-character green "Hello, World!") code completion. Inspect representative PNG/GIF frames with view_image.

- [ ] Step 5: Run media checks and commit

Run git diff --check and individual ffprobe checks. Expected: PNGs are 1672×941 and GIFs are 1200×675 at 8fps with documented frame counts. Then:

~~~text
git add -- README.md README.zh-CN.md PRODUCT.md DESIGN.md .impeccable/design.json docs/media release-notes/v0.1.0.md task_plan.md findings.md progress.md
git commit -m "docs: publish Ainotepad product identity"
~~~

### Task 7: Update Windows packaging and local release verification

**Files:**
- Rename/modify: packaging/windows/Ainotepad.iss
- Modify: packaging/windows/README.txt, .github/workflows/ci.yml, and .github/workflows/release.yml
- Modify: .gitignore only when required by new dist paths

**Interfaces:**
- Inno uses AppName=Ainotepad, AppVersion=0.1.0, {localappdata}\Programs\Ainotepad, and ainotepad.exe.
- Assets are Ainotepad-Setup-0.1.0-win-x64.exe, Ainotepad-Portable-0.1.0-win-x64.zip, and SHA256SUMS.txt.
- CI runs formatting, serial workspace tests, and cargo build --release -p ainotepad.

- [ ] Step 1: Write the packaging contract check

Assert the Inno/workflow text contains Ainotepad, ainotepad.exe, and both new asset prefixes, while current packaging files contain no Aitext product name.

- [ ] Step 2: Run RED

Run the check before edits. Expected: it fails because current files still use Aitext names.

- [ ] Step 3: Update Inno and workflows

Rename the script, update source/output paths and artifact names, retain per-user unsigned installation, and change build commands to cargo build --release -p ainotepad.

- [ ] Step 4: Run local release checks

Run:

~~~text
cargo fmt --all -- --check
cargo test --workspace -- --test-threads=1
cargo build --release -p ainotepad
git diff --check
~~~

Confirm target/release/ainotepad.exe exists. Do not launch it or run a local installer.

- [ ] Step 5: Commit

~~~text
git add -- packaging/windows .github/workflows .gitignore
git commit -m "build: package Ainotepad v0.1.0"
~~~

### Task 8: Hosted RC proof and public repository replacement

**External effects:** Push a temporary RC branch, rename the public repository, delete the old Aitext v0.1.0 Release/tag, publish replacement Ainotepad v0.1.0, and delete the RC branch only after evidence is green.

**Remote state:**
- Before rename: naipi11/Aitext.
- After rename: naipi11/Ainotepad.
- Temporary branch: codex/ainotepad-v0.1.0-rc.
- Final branch/tag: main / v0.1.0.

- [ ] Step 1: Record pre-mutation state

Run:

~~~text
gh repo view naipi11/Aitext --json nameWithOwner,defaultBranchRef,description,url
gh release view v0.1.0 --repo naipi11/Aitext --json tagName,isDraft,isPrerelease,assets
gh api repos/naipi11/Aitext/tags --paginate --jq '.[].name'
git rev-parse HEAD
~~~

Save the old main SHA and confirm RC artifact proof is complete.

- [ ] Step 2: Push and verify RC packaging

Push HEAD:codex/ainotepad-v0.1.0-rc, wait for the Windows workflow, download the hosted installer/ZIP/checksum, compare hashes, inspect ZIP contents, and confirm Get-AuthenticodeSignature is NotSigned. Never launch the installer.

- [ ] Step 3: Rename repository and origin

Run the authenticated GitHub API once:

~~~text
gh api -X PATCH repos/naipi11/Aitext -f name=Ainotepad -f description="Lightweight Windows-native AI text and code notepad with inline ghost-text completion."
git remote set-url origin https://github.com/naipi11/Ainotepad.git
~~~

Set approved topics and verify the renamed repository before continuing.

- [ ] Step 4: Replace main and delete only old v0.1.0

Use the exact old main SHA recorded in Step 1 as the lease value; do not use a bare force push:

~~~text
git push --force-with-lease origin HEAD:main
gh release delete v0.1.0 --repo naipi11/Ainotepad --yes
git push origin :refs/tags/v0.1.0
~~~

Verify old Release/tag absence before recreating the replacement tag. Do not delete unrelated refs.

- [ ] Step 5: Publish replacement v0.1.0

~~~text
git tag -a v0.1.0 -m "Ainotepad v0.1.0"
git push origin v0.1.0
gh run watch --repo naipi11/Ainotepad --exit-status
~~~

Verify the non-draft/non-prerelease Release contains the three Ainotepad assets. Download them and compare hashes against SHA256SUMS.txt.

- [ ] Step 6: Clean RC branch and record evidence

Delete codex/ainotepad-v0.1.0-rc, verify only the intended branch/tag remain, update progress/findings, and make a final documentation commit.

### Task 9: Final manual-testing handoff

**Files/state:**
- Repository: https://github.com/naipi11/Ainotepad.
- Release: https://github.com/naipi11/Ainotepad/releases/tag/v0.1.0.
- Installer: Ainotepad-Setup-0.1.0-win-x64.exe.

- [ ] Step 1: Read back public links

Verify GitHub metadata, README links, media paths, Release state, asset names, and hashes through the GitHub API.

- [ ] Step 2: Verify local tree

Run:

~~~text
git diff --check
git status --short --branch
git log -3 --oneline --decorate
~~~

Confirm no temporary RC directories, downloaded artifacts, API keys, or generated local config directories are staged or untracked.

- [ ] Step 3: Report handoff

Tell the user the public product/repository is Ainotepad, numeric version remains v0.1.0, the installer is unsigned, new documents default to Markdown, the selector supports the listed types, tests/build/release verification passed, and no desktop application was launched or controlled.

## Self-review

- Spec coverage: Tasks 1–4 cover defaults, extension mapping, new lexers, unified layout, selector, localization, and stale completion invalidation. Tasks 5–7 cover package/config identity, documentation/media, installer, and CI. Tasks 8–9 cover the approved public replacement and final handoff.
- Placeholder scan: no unresolved placeholders or unspecified filenames remain.
- Type consistency: LanguageId, language_from_path, EditorLineLayout, Command::SetDocumentLanguage, AitextApp before Task 5, AinotepadApp after Task 5, migrate_legacy_config, ainotepad.exe, and all release asset names are used consistently.
- Safety: tests and RC proof precede repository rename, old Release/tag deletion, force-update, and replacement publication; no installer or desktop launch is included.
