# 24 — Fit maps and virtual tape measure

Provenance: maintainer's functional brief (ROADMAP M24: stress and strain in colour on the draped garment to find where it is tight or loose; a tape measure on the drape, circumferences and ease read where the user clicks); general garment-making knowledge (ease = garment circumference minus body circumference at the same level; a tape around the body follows its convex outline); public papers: the deformation gradient of a linear triangle and its principal stretches (Bonet & Wood, *Nonlinear Continuum Mechanics for Finite Element Analysis*, 1997; as used for cloth in Müller et al., *Strain Based Dynamics*, 2014), the viridis colour map (van der Walt & Smith, matplotlib, 2015, CC0; Nuñez, Anderton & Renslow, *Optimizing colormaps with consideration for color vision deficiency*, PLoS ONE 2018, for why a perceptually uniform, monotone-lightness map reads for everyone), closest point on a triangle (Ericson, *Real-Time Collision Detection*, 2004).

## Status (2026-11-19, M24)

Implemented: strain and gap maps from any drape (the app's GPU or CPU
drape, the headless cached one), drawn on the 3D view with a legend,
rendered headless (`r3d render --view 3d --fit-map strain|gap`), read by
agents (`fit.report`, `fit.tape`); the tape measure tool in the app.

Not yet: a map of stress (force per width; strain times the fabric's
stiffness, which differs per direction), a pressure map, a tape around
an arbitrary (not horizontal) loop, measuring between two clicked points
on the drape.

## User story
As a designer, I drape a T-shirt and want to see at a glance where it
pulls (across the shoulders? the chest?) and where it stands off the
body; then I click around the chest to read the shirt's circumference,
the body's, and the ease between them, as a tailor would with a tape.

## Behavior

### Fit maps
- **Strain** (`strain`): for every cloth triangle, how much it is
  stretched compared with its shape on the flat pattern: the largest
  principal stretch of the triangle's deformation (rest shape from the
  2D pattern, in the shown size), minus one, in percent. 0 % is the
  pattern's own size; a compressed or folded triangle reads 0 %. Each
  vertex shows the area-weighted mean of its triangles, so the colour
  varies smoothly.
- **Gap** (`gap`): for every cloth vertex, its distance to the body's
  surface (mm), as the body is at that moment (posed if it moves). The
  cloth rests a few millimetres off the skin where it touches (its
  thickness and a margin, docs/physics.md §5), so 0–5 mm reads as
  touching.
- **Colour scale.** Both maps use one sequential scale, viridis
  (dark blue-violet → teal → green → yellow), whose lightness rises
  steadily, so it reads in grey and for colour-blind viewers. Yellow
  always means tight: strain from 0 % (dark) to 20 % (yellow) and above;
  gap from 50 mm and more (dark) to 0 mm (yellow). Fixed ranges, so maps
  of different garments compare. A jersey T-shirt reads 2–6 % on
  average and up to 40 % under the arms; a woven barely passes its
  stretch limit (about 3 %, docs/physics.md §3), so it stays dark but
  where it pulls. The scale is `r3d_render::fitmap`
  (nine stops, linear in between), shown as a legend in the app.
- Works the same on a drape from the CPU or the GPU solver: the maps are
  computed from the positions, not inside the solver.

### Reading the fit (agents)
- `fit.report` (action, reads the cached drape like `render --view 3d`;
  says `drape: simulated` or `start`): per piece its largest and mean
  strain (%), the share of its area above the tight threshold
  (`threshold_pct`, default 5 %), its mean gap and the share of its
  vertices touching the body (within 5 mm); the **tight spots**: groups
  of neighbouring triangles above the threshold, the strongest first (at
  most 10), each with its piece, peak strain, area (cm²), height above
  the floor (cm) and centre on the pattern (mm, base size); and the
  **ease** at the chest, waist and hips (plus any `heights_cm` asked).
- `fit.tape` (action): the circumference at `height_cm`, of the loop
  around the body's axis, or of the loop passing nearest to `near` (a
  point on the drape, m) at that point's height; with `near`, also the
  gap between the garment and the body there.

### Tape measure
- The drape is cut by a horizontal plane at the height asked: each
  piece's triangles give segments, joined across seams into loops. The
  **garment circumference** is the length of the cloth along its loop
  (folds included: the fabric there, as a flat measure of the garment
  would read). A loop whose two ends are less than 2 cm apart counts as
  closed; an open one (a front left open, a hem crossing the plane)
  gives its length but no ease.
- The **body circumference** is the tape around the body's loop inside
  the garment's: the convex outline of the section, as a tape laid
  around the body reads it (over the bust, not into the hollow between
  the breasts). The torso inside a shirt's body, an arm inside a sleeve.
- **Ease** = garment circumference − body circumference (cm), positive
  when the garment is larger.
- **Chest, waist, hips** come from the fitted body: the chest at the
  bust points' height, the waist at its narrowest girth, the hips at
  their largest (the heights `avatar.get` gives).
- **Gap at a point**: the distance from the clicked cloth point to the
  nearest body surface (mm).

## UX
- In the 3D view's header, a **Fit** button (a thermometer-like gauge)
  opens a menu: Fabric colours (off), Strain, Gap to the body; the chosen map colours the
  garment, and a legend appears at the bottom right of the view: the
  colour bar, its two ends with their values ("0 %" … "20 %+", or
  "50 mm+" … "0 mm"), what yellow means ("tight"), and the largest
  value on the garment and its piece.
- **Tape measure** tool (rail, under Draw on garment; no key: M already
  maximizes a view): click on the draped garment; the
  loop around it at that height is drawn on the cloth, and a card at the
  top left of the 3D view reads the height, the garment's and the
  body's circumferences, the ease and the gap at the point. Dragging
  still turns the view; the card's × or another tool clears the tape. The values
  are taken when you click: click again after the drape has moved.

## Acceptance criteria
- A rectangle stretched by 10 % along one side reads 10 % strain on
  every triangle, rotated or not; at rest it reads 0 %.
- A cylinder of cloth around a cylinder body slices into one closed
  loop of the right length; the ease equals the difference of
  circumferences; the gap at a point is the radial distance.
- On the draped T-shirt: the strain map is finite, at most a few tens
  of percent, highest at the shoulders and chest; `fit.report` returns
  every piece and the ease at chest, waist and hips; the chest ease is
  positive.
- `r3d render --view 3d --fit-map strain` writes a PNG in colour;
  `--fit-map gap` too; an unknown map is refused.
- In the app, choosing a map shows the legend; a click with the tape
  tool shows a reading (integration test).
