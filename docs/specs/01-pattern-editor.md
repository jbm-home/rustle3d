# 01 — 2D pattern editor

Provenance: maintainer's functional brief (2026-11-14); general garment-making knowledge (panels, notches, grainline, seam allowance); open SVG path semantics for cubic Béziers.

## User story
As a designer, I draw and edit flat pattern panels precisely and quickly, so
that they can later be sewn and draped.

## Behavior
- **Create panels**: rectangle (drag or typed W×H), polygon (click points),
  free contour with cubic Bézier segments (click = corner, drag = smooth).
- **Edit**: select and move points/edges/panels; convert an edge between line
  and curve; edit Bézier handles; insert/delete points; split an edge.
- **Symmetry**: mirror a panel or half-panel along an axis. A linked mirror
  copy updates live with the original (can be unlinked).
- **Notches** on edges (position by distance from a point) and **internal
  lines** (open polylines/curves inside a panel, usable later as fold or
  seam lines).
- **Grainline** per panel (an angle) that sets the warp direction for the
  simulation.
- **Grid and snapping**: grid (adjustable), snap to points, edges, midpoints,
  grid and angles (15° steps with Shift).
- **Measurements**: edge length, panel area, distance between two points;
  units mm/cm/in (display only, storage is mm).
- **Undo/redo** of every action (through the command layer).

## UX
- One default selection tool. Other tools return to selection after one use;
  double-clicking a tool locks it.
- While dragging, the length/angle appears next to the cursor; `Tab` lets you
  type an exact value.
- Hover highlights what a click will select; the status bar shows the hint
  for the current tool.

## Acceptance criteria
- A 500×700 mm rectangle can be created by typing its size, and its edges
  measure exactly 500 and 700 mm.
- Moving a point of a linked symmetric panel updates the mirror in the same
  undo step.
- Every editing command has an apply/undo/redo round-trip test.
- Snapping to points and edges works at any zoom level (pixel tolerance, not mm).
- 10 panels with 200 points total edit at 60 fps.
