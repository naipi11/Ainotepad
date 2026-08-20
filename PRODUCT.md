# Product

<!-- impeccable:product-schema 1 -->

## Platform

Native Windows desktop.

## Stack

Rust with egui/eframe, a custom text painter, Win32 locale and IME integration, and no WebView.

## Users

A Windows user writing Chinese or English notes and light source files, often switching IME and expecting one fast continuation directly at the caret.

## Product Purpose

Ainotepad is a lightweight notepad with inline ghost-text completion. Saved profiles support DeepSeek, OpenAI, xAI, Anthropic, and custom endpoints through the adapter each endpoint requires. The core loop is open, type or IME-commit, preview one suggestion, accept with Tab or dismiss with Esc, then save normally.

## Positioning

Not an IDE and not a chat sidebar. Ainotepad stays a single native process and treats completion as green preview text that never becomes document content until the user accepts it.

## Operating Context

The Paper Cut structure remains stable while its chrome follows the selected light or dark theme family. White and macOS Light use light chrome; dark editor themes use dark chrome. White is the new-install editor theme. The interface can follow Windows, use Simplified Chinese, or use English; the selection updates immediately and persists when settings are saved. Enter inserts a newline in the editor and never activates the File menu.

## Capabilities and Constraints

- Multiple tabs, open/save, undo/redo, find/replace, line numbers, and type-aware syntax highlighting.
- New documents default to Markdown; the compact status rail lets users switch among Plain Text, Markdown, and mainstream programming languages without changing file contents.
- Multiple isolated API profiles with DPAPI-backed secrets and selectable models.
- One ghost suggestion, never a completion list.
- v1 has no file tree, LSP, terminal, plugin system, WebView, or chat sidebar.
- The portable Windows executable keeps native title controls and launches without a console window.

## Brand Commitments

Name: Ainotepad. Voice: concise, technical, and consistently localized. The visual signature is a precise neutral instrument around the page, a blue focus line, a compact neutral-gray gutter divider, and green completion text. Do not copy Helix or Notepad++ source.

## Product Principles

- The page is the page: chrome recedes and typed text leads.
- Completion is preview, never document text, until Tab.
- Chinese IME, caret position, and immediate language switching are first-class.
- Familiar notepad affordances beat invented IDE controls.
- API keys never enter configuration files, logs, prompts, or interface copy.

## Accessibility & Inclusion

Keyboard focus is visible, body and secondary shell text meet contrast targets, Chinese expansion fits the settings layout, and High Contrast may override decorative Paper Cut tokens. Enter, Tab, Esc, IME composition, and native Windows window behavior keep their established meanings.
