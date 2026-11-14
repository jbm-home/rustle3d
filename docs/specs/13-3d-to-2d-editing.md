# 13 — 3D → 2D editing

Provenance: maintainer's functional description (2026-11-14); general garment-making knowledge (lengthening, taking in seams, style lines); barycentric mapping between a mesh and its 2D rest shape (standard geometry).

## User story
As a designer, I adjust the garment where I see it, on the body in 3D, and
the pattern updates itself, so I never have to work out which pattern
point to move.

## Behavior
- The 2D pattern stays the single source of truth. A 3D gesture is mapped
  to pattern coordinates (panel, position in the 2D rest shape) and turned
  into ordinary pattern commands. The cloth then re-drapes.
- Tools:
  - **Length**: drag a hem, sleeve end or neckline in 3D; the matching
    pattern edge moves along the grain.
  - **Fit**: pinch across a seam to take in, or spread to let out. Both
    sides of the seam move equally, so their lengths stay matched.
  - **Draw on garment**: a line drawn on the 3D surface becomes an internal
    line on the pattern (style line, pocket placement).
  - **Pick**: clicking the 3D garment selects the panel, edge, point or
    notch under the cursor in both views.
- Every 3D edit is undoable, appears in the history with its own label
  ("Shorten hem 30 mm") and is available to agents as commands.

## UX
- While dragging in 3D, the new length is shown next to the cursor and the
  affected pattern edge is highlighted live in the 2D view.
- The cloth keeps simulating during the gesture. There is no mode switch to
  "edit 3D".

## Acceptance criteria
- Shortening a skirt hem by 30 mm in 3D changes the pattern side seams by
  30 mm (± 1 mm) and nothing else.
- Taking in a side seam keeps both seam sides within 0.5 mm of each other.
- A line drawn in 3D maps to an internal line whose 2D points lie inside
  the panel.
- Each tool has an apply → undo → redo test at the command level.
