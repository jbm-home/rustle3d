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
  - **Length** (implemented): ⇧-drag a hem, sleeve end or neckline in 3D
    (any open edge); dragging down lengthens. The command `edge.lengthen`
    moves the edge along its panel's grain and continues across seams, so
    the whole hemline moves and seam sides stay matched; the drag is one
    undo step, a label shows the change next to the cursor, the panel is
    selected in both views and the drape follows live.
  - **Fit** (implemented): ⇧-drag a seam in 3D towards the body to take
    it in, away to let it out (twice the distance dragged). Grabbed near
    one end, only that end changes (taper: take in the waist only). The
    command `seam.take_in` moves both sides inward by half the amount;
    with linked mirror copies the symmetric seam changes too, so every
    seam stays matched.
  - **Draw on garment** (implemented): with the Draw tool (D), a drag on the
    3D garment becomes an internal line on the pattern (exact hits mapped
    to the pattern, points 8 mm apart, one panel per stroke); the stroke
    is shown under the cursor while drawing. Internal lines are not drawn
    on the 3D garment yet.
  - **Pick** (implemented for panels and edges): clicking the 3D garment
    selects the panel under the cursor (exact ray / triangle hit, mapped to
    the pattern) and the edge when the hit is within 20 mm of one; the edge
    is highlighted on the board and described in the inspector (length,
    seam partner, flip / unpick). Clicking near an edge on the board does
    the same. Points and notches: later.
  - **Same edits from the inspector** (implemented): for the picked edge,
    from either view, a typed amount (cm) with Shorter / Longer on an open
    edge (`edge.lengthen`) or Let out / Take in on a sewn one
    (`seam.take_in`), for exact values and for users who do not know the
    ⇧-drag.
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
