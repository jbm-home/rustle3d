# 02 — Sewing

Provenance: maintainer's functional brief (2026-11-14); general garment-making knowledge (seams join edges of equal length, edge direction matters).

## User story
As a designer, I tell the software which edges are sewn together, and I see
immediately whether the seam is consistent.

## Status (2026-11-15)

Implemented: whole-edge seams (`seam.add` / `remove` / `flip`, and
`seam.take_in` for fitting); `b.reversed` left out is chosen so both sides
walk the same way on the pattern; the Sew tool (S) sews the two edges
clicked on the pattern; seams are drawn on the board, both sides in one
colour; the inspector shows the picked edge's seam partner and length
difference, with flip / unpick. Not yet: partial edges and chains,
mismatch badges and direction arrows on the board, welding.

## Behavior
- A **seam** joins side A and side B. Each side is an edge segment, a part of
  an edge (between two points or notches) or a chain of consecutive segments.
- Each side has a **direction**. The seam pairs A's start with B's start,
  unless it is flipped.
- **Length mismatch** = |len(A) − len(B)|, shown in mm and as a percentage.
  Mismatches are allowed (easing/gathering), but highlighted above a threshold
  (default 3 mm).
- Seams can be edited (flip, change a side), deleted, renamed and colored.
- A free edge can take part in several seams on different sub-ranges; an
  overlap of the same range in two seams is a validation error.
- In the simulation, seams pull paired vertices together (see 04).

## UX
- **Seam tool**: click side A (hover previews the range; click and drag
  along an edge to pick a partial range), then side B. Thin colored lines
  join the two sides on the board. The same color is used in 3D.
- Direction is shown with small arrows. If the seam lines cross (a likely
  twist), a "Flip" button appears on the seam.
- Mismatch is shown as a badge on the seam (e.g. `+4 mm`), colored by
  severity.

## Acceptance criteria
- Sewing two equal 600 mm edges shows 0 mm mismatch; flipping one side swaps
  the pairing without changing the lengths.
- A partial seam between two notches uses exactly the arc length between
  them.
- Validation rejects a seam whose side overlaps an existing seam on the same
  range.
- In simulation (M6), a simple two-panel tube closes with a gap < 1 mm.
