# 11 — Templates and guided flow

Provenance: maintainer's functional brief (2026-11-14); general garment-making knowledge (basic garment blocks drafted from body measurements).

## Status (2026-11-16)

Implemented: T-shirt, dress, straight skirt, A-line skirt and tote bag as
Rust recipes in `r3d_commands::templates` (`template.apply`,
`template.list`; sizes in cm, filled in from the avatar); the start screen
with rendered thumbnails and a sizes sheet per body; the step bar with a
goal, an action and a highlighted tool per step; the Learn panel (task
cards; "Show me" points at the views, tools and panels involved with
the tour's coach marks, and never changes the project; keys and button
names in bold);
a first-launch tour. Not yet: sizes on a body silhouette (plain fields for
now). Templates are recipes in code
rather than data files: the drafts compute curves and lengths (e.g. the
sleeve cap is solved to the armhole's length).

## User story
As a beginner, I make a simple skirt or T-shirt in under 10 minutes without
knowing pattern making, and I learn along the way.

## Behavior
- **Starter templates** (our own drafts): straight skirt, A-line skirt,
  T-shirt, simple dress, tote bag. Each one is a parametric recipe: given a
  few measurements (waist, hip, length, chest…) it generates panels, notches,
  seams, arrangement and a default material, through ordinary commands.
- **Guided flow** for new garments: a step bar *Measurements → Pattern → Sew →
  Fabric → Drape*. Each step highlights its tools and shows a one-line goal.
  Steps can be skipped, and the flow can be closed at any time.
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
