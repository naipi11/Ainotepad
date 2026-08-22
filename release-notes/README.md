# Release notes

Release notes are English-only so every GitHub Release has one concise, searchable record.

Before pushing a version tag, create a matching file named `release-notes/vX.Y.Z.md`. Include:

- a one-paragraph summary;
- the user-visible highlights;
- important compatibility or security notes;
- the installer, portable package, and checksum assets when relevant.

The Windows release workflow reads the file that matches the pushed tag and publishes it as the Release body.
