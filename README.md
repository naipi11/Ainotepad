# Aitext

Aitext is a Windows-native light code notepad with one ghost-text completion at the caret. The user supplies an OpenAI-compatible Base URL, API key, and model name.

helix_copilot, Helix, and Notepad++ were behavior references only. Their source is not vendored here.

## What v1 is not

No file tree, plugins, LSP, debugger, terminal, multi-caret, minimap, regex find, Copilot login, or installer.

## Build on Windows

```powershell
cargo test --workspace
cargo build --release -p aitext
```

The portable binary is `target\\release\\aitext.exe`.

## Configure completion

Open Settings (`Ctrl+,`) and set:

- Base URL, for example `https://api.openai.com/v1`
- Model name
- API key

The key is stored under `%APPDATA%\\Aitext\\` with Windows DPAPI. It is never written into `config.toml`.

## Ghost-text keys

- Tab accepts the visible suggestion
- Esc rejects it
- After a short pause, one gray continuation may appear at the caret

## Config location

`%APPDATA%\\Aitext\\config.toml`

## License

MIT. Copyright 2026 naipi11.
