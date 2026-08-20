Aitext Windows packaging

The release workflow builds the application first, then compiles Aitext.iss
with Inno Setup on a GitHub-hosted Windows runner.

Expected input:
  ..\..\target\release\aitext.exe

Expected output:
  ..\..\dist\Aitext-Setup-0.1.0-win-x64.exe

The installer is intentionally unsigned for v0.1.0. Windows SmartScreen may
show an unknown-publisher warning.
