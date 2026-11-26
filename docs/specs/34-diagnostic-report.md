# 34 — Diagnostic report of a drape

Provenance: maintainer's functional brief (2026-11-26, ROADMAP M41: mark the faces where a drape goes wrong and get a report an agent can read: stretch and area per face, nearby contacts, seam gaps, the most strained faces, with the project and its drape state for replay; a command for the CLI and MCP first, marking in the app later); the deformation gradient and principal stretches already used by the fit maps (Bonet & Wood 1997; Müller et al., Strain Based Dynamics, 2014; spec 24); shortest paths along the cloth (Dijkstra, *A note on two problems in connexion with graphs*, 1959), for telling a face's own neighbourhood from other cloth; the signed distance field of the body (docs/physics.md §5).

## Status (2026-11-26, M41)

Implemented: the `drape.diagnose` action (MCP tool `drape_diagnose`
from the registry), `r3d diagnose`.

Not yet: marking the faces with the mouse in the app's 3D view and
copying the report for an agent; a signed distance to a moving body.

## User story
As an agent (or a developer) looking at a drape that looks wrong, I
point at where it goes wrong, on a piece of the pattern or on the drape,
and read why: the cloth is overstretched, crumpled, inside the body,
lying on another layer, or a seam has stayed open. With no place given,
the report finds the worst faces itself.

## Behavior
- Reads the cached drape (`simulate` first), like `fit.report`; before
  any drape it reports on the starting positions and says so (`drape`:
  `simulated` or `start`).
- **Regions** (optional, any number): `{"kind": "pattern", "panel",
  "at_mm", "radius_mm"}`, a disc on a piece's pattern (its coordinates,
  base size, as `fit.report` places its tight spots), or `{"kind":
  "drape", "point_m", "radius_mm"}`, a ball on the drape (m, x right, y
  up, z forward). A face belongs to a region when its centre lies in it.
  Radii 0–2000 mm (0 excluded); an unknown piece is refused
  (`not_found`).
- **A face** is reported with: its piece (id and name) and its index
  among the piece's triangles; its centre on the pattern (mm) and on the
  drape (m), and its height (cm); its **strain** (largest principal
  stretch beyond the pattern, %, as the fit maps); its **area ratio**
  (area on the drape over area on the pattern: below 1 the cloth is
  crumpled or folded flat, above 1 stretched both ways); its **distance
  to the body** (mm, its closest corner; negative inside when the body
  stands still, from the body's distance field; unsigned while the body
  moves, `body_signed` says which); its **contact**: the nearest other
  cloth within 10 mm of its centre that is not its own neighbourhood
  (its piece and distance), where the neighbourhood is the cloth within
  3 cm of the face along the pattern, crossing seams at no cost (a sewn
  piece is not a contact; a fold of the same piece, a sleeve on the
  body, another garment are); and the **largest gap of a seam** at its
  corners (mm), when it is on one.
- **Per region**: its count of faces, its largest and mean strain, its
  smallest distance to the body, its largest seam gap, and its faces,
  the most strained first.
- **Always**: counts (faces, particles, particles that are not numbers,
  particles more than 2 mm inside a still body); every seam with its
  pieces, largest and mean gap (mm), the widest first; the worst faces
  of the whole drape: the most strained, the most crumpled (area ratio
  below 1, smallest first), the deepest inside the body (beyond 2 mm);
  and `replay`: the project version the drape belongs to (its
  fingerprint, the cache's key), the simulated time (s) and the motion,
  so the same drape is found again (`simulate` with the same frames and
  motion on that version).
- `limit` (1–200, default 20) caps every list.

## Commands / API
- Action `drape.diagnose` `{regions?, limit?}` (read-only, no undo
  step); MCP tool `drape_diagnose`.
- CLI: `r3d diagnose <project> [--limit N] [--regions JSON]`, the JSON
  array of regions as the action takes it.

## Acceptance criteria
- On a draped T-shirt: no particle is not a number, almost none inside
  the body; every seam is listed, the widest first and under 10 mm; the
  strained faces come worst first; the worst face is found again by a
  1 mm disc on its pattern and by a 1 mm ball on the drape.
- Around a seam, the piece sewn on is not reported as a contact.
- A limit of 0, a radius of 0 and an unknown piece are refused.

## Out of scope
Marking in the app (later); fixing the drape: the report says what is
wrong, the agent decides what to change.
