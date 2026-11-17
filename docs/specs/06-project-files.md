# 06 — Project files

Provenance: maintainer's functional brief (2026-11-14, project options
2026-11-17); JSON (RFC 8259); zip (PKWARE APPNOTE).

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
- Project options, from a project's card on the start screen (the project
  is closed):
  - **Rename**: `project.rename`, then the folder is renamed `Name.r3d`
    (or `Name 2.r3d`, … when taken). A project already in one of those
    folders stays where it is.
  - **Delete**: removes the folder for good, after a confirmation whose
    Delete button stays disabled until an unticked "I understand" box is
    ticked. A folder without `project.json` is never deleted.
  - **Export as .zip**: the folder as `Name.r3d/…` in a zip archive,
    without `.r3d-cache/` nor temporary files.
- **Import .zip** unpacks an archive into the projects folder (a free
  `Name.r3d`) and opens it. The archive holds one `project.json`, at the
  top or in one folder. Entries that would land outside the folder,
  links, and more than 2 GiB unpacked are refused; a project that does
  not open is removed again, so nothing half-imported is left.
- Implemented in `r3d_commands::projects`, used by the app through
  `r3d-ffi`.

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
- Export then import gives a byte-identical `project.json`; archives that
  are not zip files, hold no project, or escape the folder are refused
  with an error code (`archive.invalid`, `archive.no_project`,
  `archive.too_large`) and leave nothing behind.
