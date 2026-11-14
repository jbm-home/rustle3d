# 06 — Project files

Provenance: maintainer's functional brief (2026-11-14); JSON (RFC 8259).

## User story
As a user, I save my work and reopen it later, even with a newer version of
the app, and I can read or version the file with ordinary tools.

## Behavior
- A project is a **folder** `Name.r3d/` containing `project.json`, `avatars/`,
  `exports/` and a disposable `.r3d-cache/` (see `docs/architecture.md` §3).
- `project.json` is pretty-printed with a stable key order, so diffs stay small.
- A `schema_version` field; older versions are migrated on load. Files newer
  than the app are refused with a clear message, never half-loaded.
- Save, Save As (copies assets), Open, Recent projects, autosave to
  `.r3d-cache/autosave.json` every 2 minutes, and recovery after a crash.
- Imported assets (avatars) are copied into the project folder and referenced
  by relative path.

## UX
- An unsaved-changes dot in the title. Closing with unsaved changes asks
  Save / Don't save / Cancel.
- Opening a project from an older version is silent. A backup of the original
  file is kept as `project.v{N}.json.bak` (to implement together with the
  first real migration, schema v2).

## Acceptance criteria
- Golden-file tests: each schema version's sample loads and migrates to the
  current model; save → load → save is byte-identical.
- A corrupted `project.json` produces an error message and leaves the file
  untouched; the app does not crash (fuzzed loader).
- Moving or renaming the project folder keeps it valid.
