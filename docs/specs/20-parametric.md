# 20 — Parametric patterns

Provenance: maintainer's functional brief (ROADMAP M28, 2026-11-18: a dimension can be a formula of the body's measurements and named variables, `bust / 4 + 20 mm`; the pattern redrafts when the body changes and in every size; formulas do not replace grade rules); general garment-making knowledge (drafting from body measurements plus ease); dimensional analysis (units checked as values are computed).

## Status (2026-11-18, M28)

Implemented: formulas on dimensions (`formula` on `distance`, `length`
and `angle` constraints; `constraint.set_formula`), named variables
(`variable.set`, `variable.remove`, `variable.list`), redrafting when the
body or a variable changes, each size of a range drafted with its own
measurements (the points the formulas move get their grades from them).
Project files: schema v12 (`variables`). The app: a formula button on
each dimension in the inspector, a Variables section (nothing selected).

Since M33 (2026-11-23, spec 29) formulas also name fifteen other
measurements (`neck`, `arm_length`…), typed or estimated. Not yet:
formulas on grade rules. Seamly2D and
Valentina patterns and measurement files come in through spec 28
(2026-11-20): their own drafting steps are run for each body, not turned
into constraints. Since M32 (2026-11-18) a formula can also be typed in place of
a value in the dimension's field (anything that is not a number is
taken as a formula).

## User story
As a designer, I draft a skirt whose waist is `waist / 4 + 10 mm` and
whose hem is `hips / 4 + ease`; when I change the body's measurements,
or look at another size, the skirt is drafted again for that body, and
the rest of what I fixed (lengths, angles) stays.

## Behavior
- **Formulas.** Arithmetic (`+ - * /`, parentheses, `min`, `max`, `abs`,
  `sqrt`) on numbers with units (`mm`, `cm`, `m`, `in`; `deg` or none for
  plain numbers), the body's measurements (`height`, `bust`, `waist`,
  `hips` and those of spec 29, as lengths) and named variables. Units are checked as the
  formula is computed: lengths add to lengths, `waist / 4` is a length,
  `waist + 2` is refused. A distance's or a length's formula must give a
  positive length, an angle's a plain number from 0 to 180 (degrees).
- **Errors** carry a code and are refused before anything changes:
  `formula.syntax` (with the character; also past 32 nested
  parentheses or signs, or 256 terms), `formula.unknown_name`,
  `formula.no_body` (a measurement while the project has no body),
  `formula.cycle` (a variable computed from itself), `formula.unit`,
  `formula.domain` (a division by zero, the root of a negative),
  `formula.range` (a value the dimension cannot have),
  `formula.bad_name` (a variable's name).
- **Variables** are names for formulas (`ease = 2 cm`, `dart = (hips -
  waist) / 8`), project-wide. A variable still used cannot be removed,
  nor changed so that a variable using it no longer computes.
- **Redrafting.** After every step, each dimension's formula is computed
  again (the body or a variable may have changed) and the drawing solved
  (spec 19). Without a body, formulas naming measurements keep their
  value until one is set.
- **Sizes.** With a size range (spec 18), each size other than the base
  is drafted from the base drawing: the formulas computed with that
  size's body, the constraints solved again; where each point lands is
  its grade in that size, and a smooth point's handles' too. Only the
  groups of constraints with a formula on the body are graded this way
  (their fixed points keep their own grade rule); grade rules stay for
  everything else (imported patterns, templates). A point graded by
  formulas cannot be graded by hand (`invalid.graded_by_formula`).
- Everything else follows as with any grade: the board's graded nest,
  the exports, `grading.show_size` draping a size on its own body,
  `panel.get` with `size`.

## UX
- In the inspector's Constraints section, each dimension has a ƒ button:
  a formula typed there replaces the value (shown under it, `= waist / 4
  + 10 mm`); emptied, the dimension keeps its value.
- With nothing selected, the inspector lists the Variables: add one with
  +, click one to change it, × to remove it, each with its value.

## Acceptance criteria
- A quarter skirt with `waist / 4 + 10 mm` at the waist: 190 mm for a
  72 cm waist, 210 mm after the body is changed to an 80 cm waist; one
  undo step back.
- Graded 36–44 on the women's chart, each size's waist edge is that
  size's `waist / 4 + 10 mm`, and its fixed length (600 mm) stays.
- Wrong formulas are refused with their code and nothing changes.
- Files of schema v11 open (migration), v12 golden files load.
