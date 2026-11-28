# 38 — Sizes and units of the user's region

Provenance: maintainer's functional brief (2026-11-28: a size such as 38
means nothing to some users; sizes written as S, 38 or 48 by EU, US or
another system, and lengths in mm/cm or in/ft, chosen or configured as
simply as possible; a choice made once is preselected the next time;
done in two steps to avoid regressions); general garment-making
knowledge (common women's and men's size conversion tables between
European, American, British and letter sizes).

## Status (2026-11-28, M48 done)

Step 1: the size system everywhere a size is shown (new-project window,
Body panel, the Sizes section and its chart window); the units for body
measurements (new-project window with the template's measurements, Body
panel, More measurements).

Step 2: every other length of the app follows the units: the board's
dimensions and live measurements, the inspector (sliders, fields,
labels, areas), the action bar's windows, the tape measure and the 3D
drag labels, the body check's points, the marker (fabric width in
inches, length and fabric to buy in yards), the projector (an 8 in
calibration square and a 4 in scale bar and step in inches).

Not converted: exported files (tech pack PDF, DXF, SVG, PDF pattern)
stay metric with their unit in each label; the game export's shell
thickness stays in mm (game engines work in metric); the photo lens
(mm) and place size (m) keep their usual units; fabric weight stays in
g/m² and its price per metre. A size range keeps its European names in
the project file and in exports (DXF `Size:`, tech pack).

## User story
As a designer in the United States, I start a project and see sizes
written 4, 6, 8 and measurements in inches; a friend in France sees 36,
38, 40 in centimetres. If I prefer letters, I pick S·M·L once and the
app shows them from then on.

## Behavior
- **Size systems.** Adult sizes have a name in four systems (approximate,
  as common conversion tables give them): **EU** (the charts' own, spec
  18), **US** (women: EU − 32, 38 → 6; men: the chest in inches,
  EU − 10, 50 → 40), **UK** (women: EU − 28, 38 → 10; men as US) and
  **letters** (women: 32 XXS, 34 XS, 36 S, 38–40 M, 42–44 L, 46–48 XL,
  50–52 XXL, 54–56 3XL; men: 44–46 S, 48–50 M, 52–54 L, 56–58 XL,
  60 XXL), each built-in letter preset (Women M…) being the letter of
  its own nearest size. Children's sizes are their age everywhere.
- **Units.** Centimetres (and millimetres where the app uses them) or
  inches; metres or yards for the fabric to buy; cm² and m² or in² and
  yd² for areas. Values are stored in mm/cm as before: only what is
  shown and typed changes. Inches get one more decimal than centimetres
  and two more than millimetres, at most two (a quarter of a
  millimetre). A value shown rounded in inches (34.6 in) is kept exact
  until it is typed in; a typed one is kept to the hundredth of an inch
  (a quarter inch is 6.35 mm, not 6).
- **First launch.** Guessed from the system's region: United States
  (and Liberia, Myanmar) US sizes and inches; Canada US sizes in
  centimetres; United Kingdom, Ireland, Australia, New Zealand UK sizes
  in centimetres; elsewhere EU sizes in centimetres.
- **Remembered.** A change in any window applies at once everywhere and
  is kept in the library folder (`region.json`): the next launch starts
  with it, not with the guess.

## Commands
- `grading.charts` gives each adult size `labels`: `{eu, us, uk,
  letters}`. Commands still take the EU names.

## UX
- Beside the **Size** menu (new-project window) and the range menus of
  the Sizes window, a small **EU ▾** button chooses EU, US, UK or S·M·L,
  each with examples (4 · 6 · 8). The size menu shows each size's other
  names in grey: "6   EU 38 · M".
- After the body measurements (new-project window, Body panel) a small
  **cm | in** switch. Fields' units and error messages follow it.

## Acceptance criteria
- In US sizes the new T-shirt starts on a 6; in inches its bust reads
  34.6; typing 36.2 says "Your measurements make a size 8".
- Quit and reopen: the window starts in US sizes and inches.
- In inches, a 20 × 40 cm piece's sewing line reads 47.24 in, its 10 mm
  seam allowance 0.39 in; typing 0.25 sets 6.35 mm.
- `grading.charts`: the women's 38 is US 6, UK 10, M; the men's 50 is
  US 40, M.
