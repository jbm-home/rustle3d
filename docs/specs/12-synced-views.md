# 12 — Synchronized 2D/3D views

Provenance: maintainer's functional brief (2026-11-14; maximized views 2026-11-18).

## User story
As a designer, I see the pattern and the garment side by side, and I always
know which piece of cloth corresponds to which panel or seam.

## Behavior
- Selection and hover are shared session state. Selecting a panel, edge,
  point or seam in either view highlights it in both.
- Clicking cloth in 3D selects the panel under the cursor (and the nearest
  edge when close to a border).
- Seams use the same color in both views.
- Camera: orbit, pan and zoom with presets (front, back, left, right, ¾) and
  "frame selection" (`F`).
- The layout can be split side by side, or either view maximized: a
  button on each view (or `M` over it) lets it take the space of both
  views (the inspector stays); the same button brings the default layout
  back. The board keeps its pan and zoom.
- While the 3D view is hidden, a running drape pauses (whoever started
  it, an agent included) and goes on when the view comes back; a drape
  paused by the user stays paused. Starting the drape brings the 3D view
  back.

## Acceptance criteria
- Selecting in 2D highlights in 3D within the same frame, and vice versa.
- Selection is not saved in the project and is not part of the undo history.
- Maximizing the board pauses a running drape; restoring the views resumes
  it (UI test `views_test.dart`).
- Agents can read and set the selection via MCP (so they can point the user
  at something).
