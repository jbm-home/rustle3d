# 29 — Body measurements

Provenance: maintainer's functional brief (ROADMAP M33, 2026-11-23: more body measurements — neck, shoulder, arm… — with care for the user's experience; creating a project does not ask for them, they are estimated and can be set or changed afterwards; a body panel grouped torso/arms/legs; the avatar follows those it can, shows the gap for the others; formulas, sizes and templates use them); general garment-making knowledge (where each measurement is taken); our own body model (`r3d_avatar`, MakeHuman CC0 assets) measured to calibrate the estimates.

## Status (2026-11-23, M33)

Implemented: fifteen measurements besides height, bust, waist and hips,
estimated until typed (`avatar.set_measures`, `avatar.get`), named in
formulas, graded in sizes, followed by the 3D body where it can, used by
the T-shirt and shirt templates (neckline, sleeves). The app: the Body
panel lists them by group, each with where the tape goes. Project files:
schema v20 (`avatar.measures`).

Not yet: the measurements in the user's own body presets; reading them
from Seamly2D/Valentina measurement files (spec 28 still turns them into
variables); wrist, knee, ankle, front waist length, waist height and
crotch depth on the 3D body (the body model has no targets for them).

## User story
As a designer, I start a project with a preset or my four measurements,
as before. Later I open the Body panel and type my client's neck and arm
length; the avatar's neck and arms change, the shirt's sleeve drafted
from `arm_length` lengthens, and every size of the range follows.

## Behavior
- **The measurements** (cm), by group, with their names in formulas and
  commands:
  - Torso: `neck` (around the base of the neck), `shoulder_width`
    (straight across the back from one shoulder tip to the other),
    `shoulder_length` (from the side of the neck to the shoulder tip),
    `back_waist_length` (from the nape down the spine to the waist),
    `front_waist_length` (from the side of the neck over the bust to the
    waist), `waist_height` (waist to floor), `hip_height` (hips — the
    largest girth — to floor), `crotch_depth` (waist down to the crotch;
    seated, to the chair).
  - Arms: `arm_length` (shoulder tip over the elbow to the wrist bone),
    `upper_arm` (around the upper arm at its fullest), `wrist`.
  - Legs: `inside_leg` (crotch to floor), `thigh` (around the thigh at its
    fullest, just below the crotch), `knee`, `ankle` (just above the ankle
    bone).
- **Estimated until typed.** Creating a project asks for nothing more.
  Each measurement not typed is estimated from height, bust, waist, hips
  and the masculinity: linear formulas fitted to what the built-in body
  measures, so an estimate is what the avatar measures (within 2.5 cm);
  a child's have formulas of their own, linear in its age too (spec 33).
  A typed value is kept (in the project, `avatar.measures`) until it is
  reset to the estimate. Changing the four (or the preset) keeps the
  typed ones; removing the avatar drops them.
- **Accepted ranges** are generous for adults (neck 25–65 cm, arm length
  40–85 cm…; the command's error gives the range).
- **The avatar follows** typed values of `neck`, `shoulder_width`,
  `arm_length`, `upper_arm`, `back_waist_length`, `hip_height`,
  `inside_leg` and `thigh` (the body model's targets for them, fitted
  after the four). For the others, and where a target runs out, the body
  keeps its own: `avatar.get` gives what it reaches, and the app shows
  the gap. An imported body (spec 27) does not change.
- **Formulas** (spec 20) name them like the four: `neck / 2 + 1 cm`.
  These names can no longer name variables; a variable that had one is
  renamed `my_<name>` when an older project opens, its formulas
  following.
- **Sizes** (spec 18): each size has every measurement, estimated the
  same way from its own four; a typed value moves by as much as its
  estimate does between the base size and that one (a neck typed 2 cm
  above its estimate stays 2 cm above in every size).
- **Templates** read them: the T-shirt's and the shirt's neckline from
  the 3D body's neck (which follows a typed `neck`), the shirt's sleeve is `arm_length` less 2 cm (cuff included),
  the T-shirt's 0.4 of it; shoulders from the 3D body, which follows `shoulder_width`.

## Commands
- `avatar.set_measures {measures: {neck: 37.5, arm_length: null}}`: set
  (cm) or reset (null) one or more, one undo step; refused without an
  avatar (`invalid.no_body`) or out of range (`invalid.range`).
- `avatar.get` adds `measures`: for each, `name`, `group`, `cm`,
  `estimated`, `follows` (the 3D body can follow it) and `reached_cm`
  (what the built-in body measures; null on an imported body).

## UX
- The Body panel (the avatar button) keeps the preset and the four
  measurements at the top. Below, "More measurements" in three groups:
  Torso, Arms, Legs. Each row: the name, a small figure of the body with
  the tape drawn where it goes, the value (in grey and marked
  "estimated" until typed), and a reset button once typed.
- When the 3D body is more than 1 cm away from a value it should follow,
  or cannot follow a typed value, the row says what the avatar measures.
- Typing a value and leaving the field applies it (one step); the body is
  fitted again on its own thread, as for the four.

## Acceptance criteria
- A new project from a preset asks for nothing more; `avatar.get` lists
  the fifteen, all estimated, each within 2.5 cm of what the avatar
  measures.
- Typing `arm_length` 5 cm over its estimate lengthens the avatar's arms
  by 5 cm (±1); `neck` and `inside_leg` alike; the four stay.
- A shirt template drafted afterwards has a sleeve `arm_length - 2 cm`
  long;
  undo brings the estimate back.
- `neck / 2 + 1 cm` computes from the typed neck; graded 36–44, each
  size's value moves with its estimate.
- Files of schema v19 open (migration, a variable named `neck` renamed
  `my_neck`); v20 golden files load.
