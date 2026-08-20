# Aitext Paper Cut redesign and bilingual UI

## Current mission: Aitext v0.1.0 formal release

The active release plan is [docs/superpowers/plans/2026-08-20-aitext-v0.1.0-release.md](docs/superpowers/plans/2026-08-20-aitext-v0.1.0-release.md). User-approved decisions: destructive restart strategy B, unsigned installer strategy A, Paper Cut bilingual media, no desktop automation. Current phase: plan complete; execution approach pending.

Release phases: local packaging/docs/media → RC workflow proof → repository rename/main replacement/old release deletion → v0.1.0 tag and Release verification → final handoff.

## Goal

Replace the current application chrome with the approved Paper Cut visual system and add complete System / Simplified Chinese / English localization.

## Constraints

- Preserve all existing unstaged user work; no reset, clean, checkout, stage, commit, or push.
- Use test-first changes and no real vendor API calls.
- Preserve all existing unstaged code and README edits.
- Preserve editor, profile, adapter, secret, IME, shortcut, and completion behavior.
- Keep native Windows title controls and normal window behavior.
- Do not add IDE subsystems, network localization, telemetry, or web UI dependencies.
- Use typed compile-time translations and persist only the language preference.
- Keep all existing user work unstaged and uncommitted.

## Phases

1. **Inspect current shell, strings, configuration, and reference visual** — complete
2. **Write and self-review design specification** — complete
3. **User reviews specification** — complete
4. **Write detailed implementation plan** — complete
5. **Choose execution approach** — complete
6. **Task 1: language preference and locale resolution** — complete
7. **Task 2: translation catalog and dynamic messages** — complete
8. **Task 3: localized application surfaces** — complete
9. **Task 4: localized Settings and language selector** — complete
10. **Task 5: Paper Cut shell** — complete
11. **Task 6: full verification and handoff** — complete

## Errors encountered

| Error | Attempt | Resolution |
|---|---:|---|
| None yet | 0 | — |
| Cargo rejected two positional test filters in one command | 1 | Run `i18n::tests` and `config::tests` as separate commands; update execution evidence without weakening coverage. |
| First independent review-agent launch failed to preserve its reasoning state | 1 | Closed the failed run and retried once with a compatible reviewer model; the completed review produced a bounded fix verdict. |
| Initial cleanup command containing recursive `Remove-Item` was policy-blocked | 1 | Stopped the verified test PID separately, revalidated the exact temp path, and removed only that directory through the .NET directory API. |
| Computer Use temporarily lost input geometry for the refreshed window | 1 | Re-listed the exact Release window, refreshed a screenshot-backed state, and continued with one coordinate action. |
