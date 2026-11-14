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
- **Grab**: click and drag cloth in 3D to pull it (soft constraint), and
  release to let go. **Pin**: Alt+click pins a vertex region in place; pins
  are listed and removable.
- Pattern or material edits while the simulation runs update it live (the
  panel is remeshed, positions carry over).
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
