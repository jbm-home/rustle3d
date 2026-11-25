# 11 — Templates and Learn panel

Provenance: maintainer's functional brief (2026-11-14); general garment-making knowledge (basic garment blocks drafted from body measurements).

## Status (2026-11-25)

Implemented: T-shirt, shirt, trousers, hoodie, joggers, dress, straight
skirt, A-line skirt and tote bag as
Rust recipes in `r3d_commands::templates` (`template.apply`,
`template.list`; sizes in cm, filled in from the avatar); the start screen
with rendered thumbnails and a sizes sheet per body; the Learn panel (task
cards; "Show me" points at the views, tools and panels involved with
the tour's coach marks, and never changes the project; keys and button
names in bold);
a first-launch tour. The shirt (M18) has two fronts with a 15 mm band
past the centre front, buttoned down the centre front (M20: buttons on
the under front, buttonholes on the one lapped over it, left over right
on a men's body and right over left otherwise, from below the collar to
a tenth of the length above the hem, about 9 cm apart), a back and a
yoke (M34: cut 7 cm below the shoulders, its seam topstitched in two
rows; each sleeve's back cap sewn to both), long sleeves with cuffs
(each with a 2 cm lap and one button) and a stand collar, the collar
and the cuffs interfaced (medium, woven); the trousers a front and a back per leg, placed on the legs
(`left_leg`, `right_leg`), a straight-leg block without darts whose back
crotch point is lowered until both inseams match, with an 18 cm fly
zipper (at most 60 % of the centre front) above the crotch seam (M20).
The hoodie (test report 2026-11-24) is the T-shirt's block in jersey,
16 cm of ease, its long sleeves narrowing to the shirt's cuff width,
an elastic at each wrist (relaxed to the wrist + 4 cm) and along the
hem (relaxed to the hips), and a hood in two halves (21 % of the
stature high, 14 % deep), sewn down the middle and to both necklines,
the halves meeting at the centre front; the hood starts just over the
head and falls down the back. The joggers are the trousers' block in
jersey, pulled on: no waist shaping at the sides, no fly (the centre
front sewn all the way), an elastic along each piece's waist (relaxed
to the body's waist) and hem (relaxed to the ankle + 6 cm), 8 cm
shorter than the waist's height. Not yet: sizes on a body silhouette (plain fields for
now). Templates are recipes in code
rather than data files: the drafts compute curves and lengths (e.g. the
sleeve cap is solved to the armhole's length).

## User story
As a beginner, I make a simple skirt or T-shirt in under 10 minutes without
knowing pattern making, and I learn along the way.

## Behavior
- **Starter templates** (our own drafts): straight skirt, A-line skirt,
  T-shirt, shirt, trousers, simple dress, tote bag. Each one is a parametric recipe: given a
  few measurements (waist, hip, length, chest…) it generates panels, notches,
  seams, arrangement and a default material, through ordinary commands.
  In a project with a size range (spec 18), the template is drafted for
  every size's body and its points and curves graded to match.
- No guided flow: a step bar *Measurements → Pattern → Sew → Fabric →
  Drape* with a goal banner was built in M10 and removed on 2026-11-19
  (maintainer's change request): every step was one click away already
  (Body, the rail, the inspector, Simulate), so it took room and said
  nothing new. The Learn panel and the first-launch tour are the help.
- **Learn panel**: short task cards that show where each action happens
(coach marks over the window), without touching the project (no scripted
  commands).
- Templates are plain data (a recipe file plus code), so the community can add
  more.

## UX
- The start screen has three sections: Recent (the last four projects, as
  cards with a picture of their pattern; "Open…" for a project anywhere on
  disk, and "All projects" for the rest), New (template cards with
  thumbnails generated offscreen by our renderer, and a blank project),
  and Rustle3D (the version, updates and news on rustle3d.com, the
  projects folder).
- Measurements are entered on a body silhouette with labelled fields and
  sensible defaults per size (S/M/L).

## Acceptance criteria
- Each template, at default sizes, drapes in CI (reference-scene checks).
- The 10-minute test (see `docs/ux.md` §5) passes with at least three
  first-time users before v0.1.
