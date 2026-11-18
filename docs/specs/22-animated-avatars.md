# 22 — Animated avatars

Provenance: maintainer's functional brief (ROADMAP M23: a skeleton for the avatar and a few motions, walk, turn, raise arms; the cloth follows the moving body; animation assets under a redistributable licence only); MakeHuman's default skeleton and skin weights (CC0 1.0, `assets/avatars/README.md`); general knowledge of human gait and movement for the motions (joint ranges of a normal walk, arm swing against the legs); public papers: linear blend skinning (Magnenat-Thalmann et al. 1988), the signed distance field and friction of the static body (docs/physics.md §5).

## Status (2026-11-18, M23)

Implemented: a 24-bone skeleton fitted with the body (every size and
measurement), three motions (walk on the spot, half a turn and back,
arms raised), the cloth colliding with the moving skin on the CPU and
the GPU, the app's motion menu, `simulate --motion` for agents and the
CLI, renders and exports showing the body as it is at the last frame.

Not yet: walking forward along a path (the avatar walks on the spot),
arms raised straight overhead (linear blend skinning folds the armpit;
a better skinning, e.g. dual quaternions, would allow it), motions of
imported avatars (M31), a timeline to scrub through a motion, recording
a motion as a video, skinning the garment for game engines (M15).

## User story
As a designer, I want to see how my garment moves: I drape a dress, make
the avatar walk and watch the skirt swing; I raise its arms and see
whether the shirt rides up at the waist.

## Behavior
- **Skeleton.** The avatar has a skeleton of 24 bones (pelvis, five spine
  bones, neck, head; per side a clavicle, shoulder, upper arm, forearm,
  hand, thigh, shin, foot). It is fitted with the body: its joints follow
  the measurements like the skin does. Each skin vertex follows up to four
  bones (linear blend skinning).
- **Motions.** `walk`: walking on the spot, a step cycle of 1.1 s, arms
  hanging relaxed and swinging against the legs. `turn`: half a turn to
  the left over two seconds, a pause, back, a pause (6 s). `raise_arms`:
  both arms raised to about 40° above the horizontal over 1.5 s, held,
  lowered, held (4 s). Every
  motion grows from the standing pose over its first second and then
  repeats; the feet stay on the floor.
- **When it starts.** A motion starts once the garment has closed on the
  body: after 1.5 s of draping, or at once if the drape has run longer.
  Choosing another motion, or none, first brings the body back to rest
  over one second, then starts the next one: the body never jumps.
  Restarting the drape restarts the chosen motion with it.
- **The cloth follows.** The body pushes the cloth and carries it by
  friction; pattern edits while the avatar moves carry over as they do
  while it stands. The cloth collides with the skin as the views draw it
  (docs/physics.md §5, Moving body); a contact never throws the cloth
  faster than the skin moves.
- **Agents and the CLI.** `simulate` takes `motion` (`walk`, `turn`,
  `raise_arms`); the cached drape remembers it, so `render --view 3d`,
  `render --view photo` and the 3D exports show the body and the garment
  as they are at the last frame. With fewer than 90 frames (1.5 s) the
  motion has not started yet.

## UX
- In the 3D view's simulation controls, a **Motion** button (a walking
  figure) opens a menu: Stand still, Walk, Turn, Raise arms; the chosen
  one is ticked. Choosing a motion runs the drape.
- The photo and the 3D exports from the app show the body where it is
  at that moment.

## Acceptance criteria
- A moving box carries a sheet lying on it, and the sheet never goes
  inside it, on the CPU and the GPU (`a_moving_body_carries_the_cloth`).
- Every motion keeps the avatar's feet on the floor and its skin finite;
  the rest pose moves no vertex; walking swings the arms against the
  legs; raising the arms brings the hands well above the shoulders;
  turning brings the left hand to the right side.
- A dress walking (CPU and GPU), a skirt turning and a T-shirt raising
  its arms stay on the body: no particle more than 3 mm inside the moving
  skin, seams closed; the skirt's front ends up behind after half a turn;
  the T-shirt's hem rides up when the arms go up.
- `simulate` with a motion is deterministic and its render shows the
  posed body.

## Provenance notes
The skeleton's joints and skin weights are MakeHuman's (CC0), reduced
to 24 bones by `cargo xtask fetch-makehuman`; nothing of MakeHuman's
code is used. The motions are our own: joint angles written as smooth
functions of the cycle, no motion capture data.
