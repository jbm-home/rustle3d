# 02 — Sewing

Provenance: maintainer's functional brief (2026-11-14, gathers 2026-11-16); general garment-making knowledge (seams join edges of equal length, edge direction matters; gathering a longer edge onto a shorter one).

## User story
As a designer, I tell the software which edges are sewn together, and I see
immediately whether the seam is consistent.

## Status (2026-11-18, M29)

M29 made a seam side a whole edge, a run of consecutive outline edges or
an internal line, all of it or a stretch (`from_mm`, `to_mm`), and added
ease (`ease_mm`, `seam.set_ease`): see
[21-construction-details](21-construction-details.md). The Sew tool in
the app still sews whole edges; the rest of this status is M6's.

## Status (2026-11-15)

Implemented: whole-edge seams (`seam.add` / `remove` / `flip`, and
`seam.take_in` for fitting); `b.reversed` left out is chosen so an edge
and its linked mirror image meet mirrored point to mirrored point (a
centre seam; since 2026-11-17), other edges walk both the same way on
the pattern; the Sew tool (S) sews the two edges
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
- **Gathers** (M11, implemented, schema v7): a seam marked `gathered`
  joins a longer side to a shorter one on purpose (a gathered skirt into a
  waistband, a sleeve cap). Both sides get the same number of cloth
  points, so the longer side's fullness is spread evenly along the
  shorter one when it drapes. `seam.set_gathered` turns it on or off;
  `seam.list` gives `gathered` and `gathering_ratio` (longer / shorter);
  the board does not flag a gathered seam as mismatched, and the edge
  card has a Gathered switch with the ratio.

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
- A 1520 mm skirt gathered 2:1 into a 760 mm waistband drapes with its
  seams closed (< 5 mm) and no cloth inside the body.
