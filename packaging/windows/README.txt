Ainotepad Windows packaging

The release workflow builds the application first, then compiles Ainotepad.iss
with Inno Setup on a GitHub-hosted Windows runner.

Expected input:
  ..\..\target\release\ainotepad.exe

The release workflow passes the version from the pushed `vX.Y.Z` tag to
Inno Setup and writes:
  ..\..\dist\Ainotepad-Setup-X.Y.Z-win-x64.exe

The installer is intentionally unsigned. Windows SmartScreen may show an
unknown-publisher warning.
