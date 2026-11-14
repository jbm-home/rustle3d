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
- **Destructive commands** are flagged and need explicit confirmation from
  non-UI front-ends.
- `docs/commands.md` is generated from the registry.

## UX
- An Edit menu with Undo/Redo labels naming the action ("Undo Move point"),
  a history panel to jump back several steps, and a command palette over the
  same registry.

## Acceptance criteria
- Every command has an apply → undo → redo round-trip test that checks the
  document is identical (structural equality).
- A batch with one failing command leaves the document unchanged.
- A schema snapshot test fails if a command's schema changes without the doc
  being regenerated.
