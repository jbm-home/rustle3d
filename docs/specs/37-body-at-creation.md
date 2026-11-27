# 37 — The body at creation: size, silhouette and height

Provenance: maintainer's functional brief (2026-11-27: more settings of
the mannequin's build when a project is created; a cleaner link between
measurements and size, the default size being 44 while that is seldom
the reality; modern UX rules); general garment-making knowledge
(European size charts, body shapes described by their bust, waist and
hips: hourglass, pear, straight, round, inverted triangle).

## Status (2026-11-28, M47)

Implemented: everything below.

Not yet: the fifteen other measurements (spec 29) at creation (they are
estimated, then typed in the Body panel); a silhouette for an imported
body.

## User story
As a designer, I start a project for a size 38 with a pear shape, 160
cm tall: I pick Woman, 38, Pear and type the height; the measurements
follow and I can still change one, and the window tells me which size my
measurements make. Later, when I add a size range, it is built around
my body's size, not around 44.

## Behavior
- **Who, size, silhouette, height.** A body is chosen on the built-in
  charts (spec 18): Woman (32–56), Man (44–60), Girl and Boy (2–14
  years), Baby (3–24 months). A size gives the chart's measurements; a
  silhouette keeps the bust and moves the waist and hips (women:
  standard, hourglass −6 cm at the waist, pear +6 at the hips, straight
  +6 waist −4 hips, round +12 waist +2 hips, inverted triangle −6 hips;
  men: standard, inverted triangle, straight, round); children are slim
  (×0.93), standard or sturdy (×1.08). The height is the chart's,
  changed as typed. A new project starts on women's 38, the size
  patterns are commonly drafted in.
- **A body's size.** The size nearest a body is read on the chart that
  dresses it (by age and masculinity): an adult's by the bust (as charts
  size them), then the waist and hips; a child's by its height. The
  standard women's M (bust 90, waist 73, hips 98) is a 40.
- **Size ranges around the body.** Adding a size range starts on the
  chart that dresses the body, its base the body's size, two sizes below
  and three above (before: the chart's middle, 44 on the women's).

## Commands
- `avatar.size_body {chart, size, silhouette?, height_cm?}`: the body
  (for `avatar.set`) of a chart size; an unknown size is
  `invalid.unknown_size`.
- `avatar.get` gives `size` (`{chart, name}`); `grading.charts` gives
  each chart's `silhouettes` and `nearest` (the avatar's size on its
  chart, null on the others).

## UX
- The new-project window's body section: a segmented control for who
  it dresses (and **My bodies** when the user has saved some), the size
  (or the age), the silhouette as chips (a tooltip says what each
  means), then height, bust, waist and hips, filled in and editable.
  Typed values say "Your measurements make a size 40." when they leave
  the chosen size.
- A template's sizes are worked out for the body in the background as it
  changes (a thin progress bar over them); the window never blocks, and
  Create waits for them.
- The Body panel says the size nearest the measurements as they are
  typed ("Nearest size: 40 (Women)").

## Acceptance criteria
- A new T-shirt starts on women's 38 (bust 88); choosing 44 shows a
  bust of 100 and the T-shirt's sizes for it; Pear keeps 100 and makes
  the hips 114; typing a bust of 92 says the measurements make a 40.
- Choosing Baby updates the template's sizes without a card over the
  window.
- On the women's M body, the size range offered is women's 36 to 46
  based on 40; `grading.charts` gives `nearest` 40 on the women's chart.
