# 16 — Cutting a piece

Provenance: maintainer's functional brief (2026-11-16, M11 test: "a cut
tool for straight or curved cuts; once confirmed, two pieces sewn along
the cut"); general garment-making knowledge (style lines such as princess
seams split a piece into pieces sewn back together).

## User story
As a designer, I draw a straight or curved line across a piece and get two
pieces sewn together along it, so that I can add style lines (a princess
seam, a yoke) without redrawing the pattern.

## Behavior
- `panel.cut` takes an open path (straight and cubic segments, mm) that
  crosses the piece's outline exactly twice, both ends outside the piece.
  Without `panel`, the one piece the path crosses is cut (two
  candidates is an error).
- The outline is split where the path crosses it (a crossing within
  0.5 mm of a corner uses the corner); the part of the path inside the
  piece becomes an edge chain on both new pieces, with the same curves.
- The larger piece keeps the panel's ID (so its seams, pins and selection
  stay); the other is a new panel with the same garment, grain and seam
  allowance, named after the piece ("Front 2") unless `name` is given,
  moved 20 mm away from the other on the board so both show.
- One seam per cut edge sews the two pieces together (equal lengths).
- Notches stay on their edges; on a split edge they go to the half they
  are on. Internal lines and pins go to the piece they start on.
- A sewn edge that is cut is split with its partner at the same fraction
  of the seam, and the seam becomes two (gathering kept). Cutting both
  sides of one seam at once is refused.
- A placed piece's halves keep their place around the body exactly: both
  keep the whole's angle and the pattern abscissa that goes to it
  (`Placement.centre_x_mm`, schema v8; moving a piece on the board moves
  it too), and each half's top is lowered by as much as it sits below the
  whole's top. So the halves start where the whole piece would have.
- Refused: a path that does not cross exactly twice, halves that would
  cross themselves, a linked mirror copy or a piece that has one (unlink
  first).
- One command, one undo step.

## UX
- Cut tool (K) on the rail: click points across a piece from outside to
  outside, drag for a curve (as with Curve); the line is drawn in the
  danger colour; Enter cuts, Esc cancels.

## Acceptance criteria
- A 400×600 rectangle cut at x = 150 gives pieces of 250×600 (the
  original ID) and 150×600, sewn with one 600 mm seam.
- A curved cut keeps the total area, and undo restores the single piece.
- A cut across a sewn side seam gives three seams of matching lengths.
- An A-line skirt cut along a curved princess line drapes like the whole
  skirt: same waist height and flare, seams closed, no cloth in the body.
- Every case is covered by `crates/r3d-commands/tests/cut.rs`.
