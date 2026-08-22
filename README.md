<p align="center">
  <img src="crates/ainotepad/assets/ainotepad-icon.png" alt="Ainotepad icon" width="128">
</p>

<h1 align="center">Ainotepad</h1>

<p align="center">
  <strong>Ghost text at the caret.</strong><br>
  A lightweight Windows-native text and code editor for writing, notes, and code.
</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/naipi11/Ainotepad/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/naipi11/Ainotepad/ci.yml?branch=main&label=CI&logo=github"></a>
  <a href="https://github.com/naipi11/Ainotepad/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/naipi11/Ainotepad?display_name=tag&sort=semver&label=release"></a>
  <a href="https://github.com/naipi11/Ainotepad/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/naipi11/Ainotepad?style=flat&label=stars&logo=github"></a>
  <a href="https://github.com/naipi11/Ainotepad/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/naipi11/Ainotepad/total?label=downloads"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/github/license/naipi11/Ainotepad?label=license"></a>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-native-orange?logo=rust&logoColor=white">
  <img alt="egui" src="https://img.shields.io/badge/UI-egui-2f80ed">
  <img alt="Windows native" src="https://img.shields.io/badge/Windows-native-0078D4?logo=windows&logoColor=white">
  <img alt="Markdown" src="https://img.shields.io/badge/Markdown-default-6f42c1">
  <img alt="AI ghost text" src="https://img.shields.io/badge/AI-ghost%20text-43a047">
</p>

<p align="center">
  <a href="https://github.com/naipi11/Ainotepad/releases/latest"><strong>Download for Windows</strong></a>
  · <a href="#quick-start">Quick Start</a>
  · <a href="#star-history">Star History</a>
</p>

![Ainotepad Paper Cut](docs/media/hero-en.png)

![Ainotepad ghost text demo](docs/media/demo-en.gif)

Ainotepad is a lightweight Windows-native text and code editor with one inline ghost-text continuation at the caret. It keeps the writing surface clear, supports Chinese IME, and connects to provider profiles without becoming an IDE or chat sidebar.

## Quick Start

### Install

Download the [Ainotepad installer](https://github.com/naipi11/Ainotepad/releases/latest/download/Ainotepad-Setup-0.1.0-win-x64.exe), or use the [portable ZIP](https://github.com/naipi11/Ainotepad/releases/latest/download/Ainotepad-Portable-0.1.0-win-x64.zip).

The v0.1.0 installer is unsigned, so Windows SmartScreen may show an unknown-publisher warning. Verify the download with [SHA256SUMS.txt](https://github.com/naipi11/Ainotepad/releases/latest/download/SHA256SUMS.txt) when needed.

### Configure a provider

1. Open **Settings** with `Ctrl+,`.
2. Choose **AI Profiles** and add or select a profile.
3. Set the provider, adapter, Base URL, model, and API key.
4. Save settings. API keys are stored with Windows DPAPI and are not written to `config.toml`.

DeepSeek, OpenAI, xAI/Grok, Anthropic/Claude, and custom endpoints are supported. Use FIM, Chat Completions, or Responses when the selected provider exposes that adapter.

### Use ghost text

Type in a document and pause briefly. A green continuation appears at the caret when a configured provider returns a suggestion.

- `Tab` accepts the visible suggestion.
- `Esc` dismisses it.
- Chinese IME composition remains separate from the document and stays anchored to the caret.

## Highlights

- Paper Cut shell with White, Black Green, VS Code, macOS Light, Dark, Lamp, High Contrast, and Custom themes.
- English, Simplified Chinese, and Follow Windows interface language modes.
- Multiple isolated API profiles and selectable models.
- Markdown is the default for new documents; the status bar type selector supports Plain Text, C/C++, C#, Python, Rust, JavaScript/TypeScript, HTML, CSS, JSON, TOML, PowerShell, Batch, and INI.
- Syntax highlighting for the selected document type, tabs, find/replace, undo/redo, and configurable status information.
- Native Windows title controls and a portable single-process editor.

## Star History

<p align="center">
  <a href="https://star-history.com/#naipi11/Ainotepad&Date">
    <img src="https://api.star-history.com/svg?repos=naipi11/Ainotepad&type=Date" alt="Star History Chart">
  </a>
</p>

## Build from source

Install the stable Rust toolchain, then run:

```powershell
cargo test --workspace -- --test-threads=1
cargo build --release -p ainotepad
```

The portable executable is written to `target/release/ainotepad.exe`.

## Scope

Ainotepad is intentionally a focused editor. It does not include a file tree, LSP, debugger, terminal, plugin system, minimap, multi-caret editing, chat sidebar, Copilot login, or runtime translation downloads.

## License

MIT. Copyright 2026 naipi11.
