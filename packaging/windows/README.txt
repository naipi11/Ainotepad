Aitext Windows packaging

The release workflow builds the application first, then compiles Aitext.iss
with Inno Setup on a GitHub-hosted Windows runner.

Expected input:
  ..\..\target\release\aitext.exe

The release workflow passes the version from the pushed `vX.Y.Z` tag to
Inno Setup and writes:
  ..\..\dist\Aitext-Setup-X.Y.Z-win-x64.exe

The installer is intentionally unsigned. Windows SmartScreen may show an
unknown-publisher warning.
