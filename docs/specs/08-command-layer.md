# 08 — Command layer and undo

Provenance: maintainer's functional brief (2026-11-14); JSON Schema (draft 2020-12).

## User story
As a user, every action I take can be undone. As an agent or a script, I can
do anything the UI can do, through the same typed commands.

## Behavior
- Every user-visible operation is a **command**: a name (`panel.create_rect`),
  typed parameters (serde), a JSON schema (schemars), validation and an
  application that records changes. See `docs/architecture.md` §4.
- **Undo/redo** replays recorded changes. Consecutive edits with the same
  merge key (e.g. dragging a point) merge into one step.
- **Batch**: a list of commands executed atomically, forming one undo step.
- **Queries**: read-only operations with schemas (summary, lists, geometry,
  measurements, simulation state).
- **Destructive commands** are flagged and need `"confirm": true` from every
  front-end (the UI sets it after its own confirmation dialog).
- **Integrity** is checked after every command; a command that would break
  an invariant is rolled back and reported as an `internal` error.
- **Errors** are structured for translation: stable `code`, named `args`,
  English `text` fallback.
- `docs/commands.md` is generated from the registry.

## UX
- Undo/Redo name the action ("Undo Move point") in their tooltips; the
  app has no menu bar, so the history button next to them lists every
  step (`history.list`), the latest first, and a click goes back or
  forward to it (`history.go_to`). A command palette uses the same
  registry.
- A step is what the user sees as one action: a drag (merged by merge
  key, sealed when the pointer is released), a tool's whole gesture, a
  template added (one command), a batch (named by its `label`).
- Undo, redo and go-to select the panels the steps changed.

## Acceptance criteria
- Every command has an apply → undo → redo round-trip test that checks the
  document is identical (structural equality).
- A batch with one failing command leaves the document unchanged.
- A schema snapshot test fails if a command's schema changes without the doc
  being regenerated.
