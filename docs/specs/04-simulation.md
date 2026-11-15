# 04 — 3D simulation

Provenance: maintainer's functional brief (2026-11-14); papers listed in `docs/physics.md`.

## User story
As a designer, I press Simulate and see my garment sew itself and drape on
the avatar in real time, and I can pull on it to check how it hangs.

## Behavior
- Real-time drape on a **static** avatar: gravity, seam forces, avatar
  collision with friction, basic self-collision.
- Controls: **Simulate** (start/resume), **Pause**, **Reset** (back to the
  arranged positions), **Step** (one frame, also for the CLI).
- **Grab** (implemented): click and drag cloth in 3D to pull it (soft
  constraint on the cloth within 3 cm of the picked point, offsets kept),
  and release to let go. A click without moving only selects the piece
  under the cursor; grabbing a paused simulation runs it.
- **Pin** (implemented): ⌥-click (Alt-click) pins the cloth
  under the cursor where it is. Pins are document entities (`pin.add`,
  `pin.remove`, `pin.list`; saved, undoable, available to agents): the
  cloth within 3 cm of the point on the pattern is held at the pin's 3D
  position, from the start of every simulation. Pins are listed in the
  inspector and removable; adding or removing one applies at once to the
  running simulation (undo/redo apply on the next Restart).
- Pattern or material edits while the simulation runs update it live
  (implemented): the scene is rebuilt at most every 100 ms and each
  particle takes the position of the same pattern point in the previous
  drape (barycentric remapping, extrapolated where a panel grew); seams
  stay closed and the simulation time carries on.
- "Save drape" stores the current shape in the project; reopening shows it
  without re-simulating.
- Quality setting: particle spacing (Draft 20 mm / Normal 10 mm / Fine 5 mm).
- Deterministic mode for tests and the CLI.

## UX
- One big Simulate/Pause button in the 3D view; `Space` toggles it.
- The status bar shows simulation fps and a calm "settled" indicator when the
  cloth stops moving.
- Problems (exploding cloth, persistent penetration) show as a hint with a
  fix ("Seam twisted? Try flipping seam *Side seam L*"), never as a crash.

## Acceptance criteria
- The reference scenes of `docs/physics.md` §8 pass in CI (no NaN, no
  penetration beyond tolerance, seams closed).
- T-shirt at ~10k particles ≥ 30 fps on the reference machine (benchmark).
- Grab and pin work at any time and never make the simulation explode
  (fuzzed in tests with random grab targets).
- Reset always restores the arranged state exactly.
