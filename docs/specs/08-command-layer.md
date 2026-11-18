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
- **Copy and paste** (M26): `panel.copy` (a query) gives a clip of pieces
  with their points, curves, notches, internal lines, pins and grade
  rules, the seams between them and the mirror links among them;
  `panel.paste` adds a clip to a project, the same or another one, under
  new IDs: to the right of the pieces by default, grade rules kept when
  the size range has the same number of sizes and base, a piece left
  unplaced (and its pins dropped) when another already starts where it
  would.
- **Long steps** (M26) report their progress and can be stopped: a
  template drafted in every size of a range (`template.apply`,
  `grading.use_chart`, `grading.set_sizes`), a headless drape
  (`simulate`). A stopped step fails with `cancelled` and changes nothing.

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
- ⌘C copies the selected pieces as a clip on the system clipboard, ⌘V
  pastes it and selects what was pasted.
- **The app never looks frozen** (maintainer, 2026-11-18): opening a
  project, adding a template, setting a size range or new measurements,
  importing, exporting and starting the drape run off the UI thread.
  Past 300 ms a card says what is being done: a bar with the size being
  drafted when the length is known, an indeterminate bar otherwise, and
  Stop where stopping is safe (not during a body fit). While a body is
  fitted for the 3D view, the view keeps the old body and says so.

## Acceptance criteria
- Every command has an apply → undo → redo round-trip test that checks the
  document is identical (structural equality).
- A batch with one failing command leaves the document unchanged.
- A schema snapshot test fails if a command's schema changes without the doc
  being regenerated.
- `history.go_to` reaches any step and back; the history lists the
  command names; undo returns the panels changed.
- Two sewn pieces pasted into another project keep their seam; a bad clip
  changes nothing (`malformed`).
- A graded template reports each size; stopped, it changes nothing and
  adds no step to the history.
