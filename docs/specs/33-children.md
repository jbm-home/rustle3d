# 33 — Children

Provenance: maintainer's functional brief (ROADMAP M39, 2026-11-25:
children's avatars, girls and boys; templates and library patterns for a
bodysuit, a pyjama and a jogging suit, and what else matters for
children's clothes); MakeHuman's CC0 assets (baby, child and young
targets, `assets/avatars/README.md`); general garment-making knowledge
(European children's size charts by age, a baby's bodysuit closed under
the crotch, pyjama and jogging sets).

## Status

In progress (M39, 2026-11-25).

- Bodies: a child's age on the avatar's body (`age_years`, 0–18; schema
  v27) and on each size of a range; the body model mixes MakeHuman's
  baby, child and adult targets by age; presets and size charts by age;
  the other measurements estimated with children's coefficients.
- Templates: a baby's bodysuit, a pyjama and a jogging suit (sets of two
  garments), a pyjama's top and bottoms, a sweatshirt; templates place
  and draft on a child's proportions.
- The app: Adult / Child and an age in the Body panel, the children's
  presets, the girls', boys' and babies' charts in the sizes dialog,
  "Babies and children" in the library, the children's templates on the
  start screen. The 3D view frames a child's body as it frames an
  adult's.

## User story

As a designer of children's clothes, I choose a 4-year-old girl or a
12-month-old baby as the avatar, add a pyjama or a bodysuit from the
library, drafted for that body, and grade it from 2 to 14 years.

## Behavior

- **A child's body**: the avatar's body takes an age in years (0 to 18;
  left out, an adult). The body model mixes MakeHuman's bodies by age as
  its own age modifier does: a baby's (1 year) and a child's (11 years)
  linearly between 1 and 11, a child's and an adult's (25 years) between
  11 and 25; younger than 1, a baby's. Its height, bust, waist and hips
  are then fitted as an adult's. Masculinity reads as girl / boy.
- **Presets** (Body panel): babies 3m, 6m, 12m, 18m, 24m (the same for
  girls and boys), girls and boys 2, 3, 4, 5, 6, 8, 10, 12 and 14 years;
  approximate values of common European charts: height, chest, waist,
  hips by age.
- **Size charts** (`grading.use_chart`): `girls` and `boys` (sizes named
  by age, 2 to 14), `babies` (3m to 24m). Each size's body differs from
  the avatar's in height, bust, waist, hips and age; without an avatar
  the chart's base size becomes it.
- **Other measurements**: a child's are estimated from coefficients of
  their own (girls' and boys', linear in the four measurements and the
  age), fitted to what the body model measures on 205 bodies of each
  sex. They agree with the 3D body within 2.5 cm, the hip height within
  3.5 cm (a child's belly is nearly as round as its hips; a baby's hip
  height is not reliable). Accepted values go down to a baby's.
- **Proportions**: a child's head is larger: templates place tops from
  its neck and sleeves from its shoulders, and draft trousers' rise to
  its crotch, as the body model's child of that age has them (the adult
  fractions scaled by the child's own); a hood follows the head; shorts
  end in proportion to a short leg.
- **Templates** (kind "Babies and children" in the library, spec 32):
  - **Bodysuit** (jersey): the T-shirt's block with short sleeves, going
    on below the hips into leg openings that curve in to a crotch 1/7
    of the hips wide, where the front and back are sewn together (a
    bodysuit's snaps; snaps are not a trim yet). Length from the neck
    down under the crotch.
  - **Pyjama**: a set of two garments, the bottoms then the top worn
    over them. The **pyjama top** (jersey): the T-shirt's block, 14 cm
    of ease, long sleeves gathered at the wrists by an elastic, a loose
    hem. The **pyjama bottoms** (jersey): the joggers' block, 12 cm of
    ease, an elastic waist, open at the ankles, 8 cm above the floor.
  - **Jogging suit**: a set: joggers, then a **sweatshirt** worn over
    them (the hoodie without its hood: elastic cuffs and hem band).
  - A set's sizes are its top's (length, ease, sleeve); its bottoms take
    the waist and hips given and their own length and ease. Each part is
    a garment of its own, with its own template, graded and drafted
    again for another body like any other.
  - Every other template drafts on a child's body too.
- **3D view**: the camera frames a body of another height as it framed
  the last one (a child comes closer); "Look from" frames it head to toe.

## UX

- Body panel: **Adult | Child**; with Child, an **Age** field (years)
  and **Girl | Boy** in place of Feminine | Masculine. The presets list
  the babies', girls' and boys' sizes after the adults'.
- Sizes dialog: **Women | Men | Girls | Boys | Babies**.
- Library: a "Babies and children" section: bodysuit, pyjama, pyjama
  top, pyjama bottoms, jogging suit. Start screen: bodysuit (on a 12
  months baby), pyjama (a 6-year-old girl), jogging suit (an 8-year-old
  boy), the body changeable in the dialog as for any template.

## Acceptance criteria

- A 4-year-old girl's pyjama: the bottoms and the top over them, draped
  on her body without the top passing into the bottoms.
- A baby's bodysuit drapes closed under the crotch.
- A pyjama graded with the girls' chart from 2 to 14, each size drafted
  on a body of that age.
- Every step reachable from the CLI and MCP (`avatar.set` with
  `age_years`, `grading.use_chart` with `girls`, `template.apply`).

## Not yet

- An envelope neckline on the bodysuit (lapped shoulders), snaps as a
  trim, binding on the leg openings.
- A child's own motions (the walk is an adult's, scaled).
- Children's bodies older than 14 are mixed with the adult's (MakeHuman
  has no teenager); a baby's hip height wanders between belly and hips.
