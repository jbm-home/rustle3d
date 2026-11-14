# 12 — Synchronized 2D/3D views

Provenance: maintainer's functional brief (2026-11-14).

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
- The layout can be split side by side, or either view maximized.

## Acceptance criteria
- Selecting in 2D highlights in 3D within the same frame, and vice versa.
- Selection is not saved in the project and is not part of the undo history.
- Agents can read and set the selection via MCP (so they can point the user
  at something).
