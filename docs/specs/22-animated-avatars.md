# 22 — Animated avatars

Provenance: maintainer's functional brief (ROADMAP M23: a skeleton for the avatar and a few motions, walk, turn, raise arms; the cloth follows the moving body; animation assets under a redistributable licence only); MakeHuman's default skeleton and skin weights (CC0 1.0, `assets/avatars/README.md`); general knowledge of human gait and movement for the motions (joint ranges of a normal walk, arm swing against the legs); natural motions (maintainer's brief 2026-11-23, ROADMAP M36: the walk hopped and moved like a stiff robot): published gait studies, D. A. Winter, *The Biomechanics and Motor Control of Human Gait* (2nd ed., 1991), Kadaba, Ramakrishnan, Wootten, J. Orthop. Res. 8(3), 1990, J. Perry, *Gait Analysis* (1992), Murray, Drought, Kory, J. Bone Joint Surg. 46-A(2), 1964, Saunders, Inman, Eberhart, J. Bone Joint Surg. 35-A(3), 1953 (shapes and ranges, rounded; no data set copied); two-bone inverse kinematics in closed form (Tolani, Goswami, Badler, Graphical Models 62(5), 2000); public papers: linear blend skinning (Magnenat-Thalmann et al. 1988), dual quaternion skinning (Kavan, Collins, Žára, O'Sullivan, ACM TOG 27(4), 2008), the signed distance field and friction of the static body (docs/physics.md §5); captured motions: maintainer's brief (2026-11-19: import motion captures in the app to replace the procedural walk, starting with a slow walk of the CMU Graphics Lab database in its BVH conversion), the BVH format as publicly described (Meredith and Maddock, "Motion Capture File Formats Explained", University of Sheffield, 2001) and our own retargeting and loop search.

## Status (2026-11-18, M23)

Implemented: a 24-bone skeleton fitted with the body (every size and
measurement), three motions (walk on the spot, half a turn and back,
arms raised), the cloth colliding with the moving skin on the CPU and
the GPU, the app's motion menu, `simulate --motion` for agents and the
CLI, renders and exports showing the body as it is at the last frame.

2026-11-19: captured motions. A BVH file imported into the project
(`motions/`) plays on the avatar, retargeted to its skeleton and
looped; in the app's Motion menu and through `simulate`'s
`motion_file`. No capture is shipped with the app. A motion library
shared by every project (the app's): add to it after an import,
manage it in a dialog, play its motions from the Motion menu.

2026-11-25 (M36): natural motions. Dual quaternion skinning; the feet
placed first and the legs reaching them by inverse kinematics, so a
planted foot neither slides nor lifts; the pelvis lowered as the legs
need, smoothly; the walk after published gait studies, a supple trunk,
a steady head, relaxed arms; the turn made in steps; the arms raised in
a V above the head.

2026-11-25 (M37): imported bodies move (spec 27); the game export
carries the project's captured motions as animations (spec 15).

Not yet: walking forward along a path (the avatar walks on the spot,
as on a treadmill), arms raised straight overhead (the body takes it,
but the shoulders' skin then carries a T-shirt's neckline in and up and
it rolls over at the chest), a standing stance narrower than the rest
pose's (idle keeps the feet where they rest), a timeline to scrub
through a motion, recording a motion as a video, glTF animations as
captured motions, the motion library for agents and the CLI (they
import files into the project).

## User story
As a designer, I want to see how my garment moves: I drape a dress, make
the avatar walk and watch the skirt swing; I raise its arms and see
whether the shirt rides up at the waist.

## Behavior
- **Skeleton.** The avatar has a skeleton of 24 bones (pelvis, five spine
  bones, neck, head; per side a clavicle, shoulder, upper arm, forearm,
  hand, thigh, shin, foot). It is fitted with the body: its joints follow
  the measurements like the skin does. Each skin vertex follows up to four
  bones by dual quaternion skinning (since M36; linear blend skinning
  before): a bent joint keeps its volume.
- **Motions.** The built-in motions place the feet first, on the floor
  or in the air, and the legs reach them by inverse kinematics (two
  bones and a hinge, the knee pointing as at rest, never straighter
  than at rest). The pelvis comes down only as far as the legs need:
  their limit over the whole cycle, eroded by parabolas of 2 m/s² and
  smoothed over 0.08 s, so it starts down early enough, rises and falls
  smoothly and never goes above its rest height. The trunk turns
  against the pelvis and keeps the shoulders level, the head is kept
  level and looking ahead (or where the motion looks), the arms hang
  relaxed from the A pose (brought 30° down, elbows straightened to
  15°, wrists a little bent).
  - `walk`: on the spot, as on a treadmill, a stride of 1 s (120
    steps a minute), a stride length of 1.2 times the hip height.
    A planted foot goes back with the belt at one steady speed: it lands
    on its heel at 18° to the floor, rolls flat by 8 % of the cycle,
    stays flat, lifts its heel from 40 % rolling about its front, and
    leaves at 62 % at 32°. The swinging foot follows the swing of
    published hip, knee and ankle curves (every 5 % of the cycle),
    bent to leave and land exactly where the planted foot is, clearing
    the floor by 12 mm. The feet are 85 % of the hips' width apart,
    turned out 6°. The pelvis drops 4° on the swing side, turns 4.5°
    with the swinging leg and sways 2 cm over the loaded foot; the
    thorax turns 3° the other way and leans 2° forward. The arms swing
    ±15° about 3° back, against the legs and a little late, the elbow
    bending up to 10° more just after the arm is forward, the wrist
    following. The knee bends about 5–7° as the foot takes the weight
    and about 50° in swing.
  - `idle`: standing at rest over 8 s: the weight shifts onto one leg
    and the other (the hips 3 cm over the loaded foot, its side of the
    pelvis 1.5° up, the other knee easing and its heel lifting a
    little, the feet where they rest), two breaths, the head looking a
    little around.
  - `turn`: half a turn to the left and back over 6 s, in steps about
    the vertical between the feet: still for half a second, round over
    two (the left foot opens a third of the way, the right follows, and
    so on, four steps of 0.45 s each lifting the foot 3.5 cm, the weight
    shifting over the foot that stays), a pause, back the same way led
    by the right foot, a pause. The head turns ahead of the trunk and
    the trunk ahead of the pelvis.
  - `raise_arms`: both arms raised over 1.5 s in a V above the head
    (about 55° above the horizontal), held 0.5 s, lowered over 1.5 s,
    held (4 s); the clavicles rise 8°, the elbows straighten to 10°,
    the chest opens a little.
  Walking and standing grow from the rest pose over their first second
  (stride, sway and arm swing); turning and raising the arms start from
  it. Every motion then repeats; the feet stay on the floor.
- **Captured motions.** A BVH file (the common motion capture format) is
  copied into the project's `motions/` folder and named by its path
  there. It is read, retargeted and looped when chosen:
  - Joints are found by their usual names, whatever the namespace or
    case: hips, spine joints up to the neck, neck, head, and per side
    collar or shoulder (optional), arm, forearm, hand, up-leg or thigh,
    leg or shin, foot, toe (optional), `Left`/`Right`, `l`/`r` before or
    after. The file's axes are found from its rest pose (up from the hips
    to the head, left from the right thigh to the left one), so Y-up and
    Z-up files both work. When the rest pose does not stand as the body
    does in the motion (its trunk or thighs more than 45° off the
    motion's up, the file axis nearest to the hips-to-head direction
    over the capture), as in files that give each joint a frame of its
    own and lay every bone along one axis, the sampled frame that stands
    best is the rest pose instead.
  - Each of our bones takes the matching joint's rotation from its rest
    pose, after a fixed turn that lays our bone onto the captured one in
    the rest pose (a captured T pose plays on our A-pose body); a hand
    or the head keeps its parent's fixed turn; our six spine bones share
    the captured spine's rotations along it. The motion is fitted to each
    body when played, so it follows every size. Fingers are not used.
  - The loop is the longest stretch (at least 1 s, at most the first
    30 s) whose end matches its start, poses and speeds compared over
    seven frames, within a tenth of the way from the best match to an
    average one (of the longest, the one starting first); its last quarter second blends into the frames before
    its start. The hips' heading over the loop loses its linear trend, so
    a walk drifting into a curve faces forward and loops. The avatar
    stays on the spot (the captured travel is dropped) with its lowest
    sole on the floor.
  - A file that cannot be read, a skeleton missing a required joint, or
    a capture too short for a loop is refused with a message saying
    which.
- **Motion library.** The user's captured motions, shared by every
  project, kept in the app's library folder (`motions/`, one BVH file
  per motion, named by the file's stem). A file is checked like an
  import before it is added; a file of the same name and other content
  is numbered (`walk (2)`), the same file again is not copied twice.
  Motions can be renamed (a name a file can have on every system:
  not empty, at most 80 characters, none of `/ \ : * ? " < > |`, no dot
  first or last) and deleted. Playing a library motion copies it into
  the project's `motions/` (as an import does), so a project never
  depends on the library; deleting it from the library leaves projects
  unchanged.
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
  `raise_arms`, `idle`) or `motion_file` (a BVH file in the project folder;
  `r3d simulate --motion file.bvh` copies the file into `motions/`
  first); the cached drape remembers it (a captured one by its path and
  a hash of the file, so a changed file makes the cache stale), so `render --view 3d`,
  `render --view photo` and the 3D exports show the body and the garment
  as they are at the last frame. With fewer than 90 frames (1.5 s) the
  motion has not started yet.

## UX
- In the 3D view's simulation controls, a **Motion** button (a walking
  figure) opens a menu: Stand still, Walk, Turn, Raise arms; the chosen
  one is ticked. Choosing a motion runs the drape.
- Under them, the project's captured motions by file name, then
  the library's motions the project has no copy of (by name), then
  **Import a captured motion…**, which asks for a `.bvh` file, imports
  it and plays it, and **Motion library…**.
- After an import, the message offers **Add to library** (offered after
  rather than asked before, so an import stays one step).
- The library dialog lists the motions, each with play (copies it into
  the project, plays it, closes the dialog), rename (in place) and
  delete (confirmed); **Add files…** adds several BVH files at once; a
  file that cannot be added says why under the list.
- The photo and the 3D exports from the app show the body where it is
  at that moment.

## Acceptance criteria
- A moving box carries a sheet lying on it, and the sheet never goes
  inside it, on the CPU and the GPU (`a_moving_body_carries_the_cloth`).
- Every motion starts from the rest pose and keeps the avatar's feet
  on the floor and its skin finite; the rest pose moves no vertex;
  walking swings the arms against the legs; raising the arms brings
  the hands above the top of the head; turning brings the left hand to
  the right side.
- Walking: the heel of a flat foot and the front of a foot rolling onto
  it go back with the belt at one speed, within 3 mm, and stay on the
  floor; the pelvis rises and falls by 1 to 4.5 cm with a vertical
  acceleration under 4 m/s² (sampled 240 times a stride); the knee
  bends 5–25° before 50 % of the cycle and 45–70° in swing; the head
  turns less than 1.5°. Standing, the front of each foot stays within
  2 mm; turning, a foot between its steps stays within 1 mm.
- A leg placed by inverse kinematics reaches its foot within 0.1 mm and
  rests at rest.
- A dress walking (CPU and GPU), a skirt turning and a T-shirt raising
  its arms stay on the body: no particle more than 3 mm inside the moving
  skin, seams closed; the skirt's front ends up behind after half a turn;
  the T-shirt's hem rides up when the arms go up.
- `simulate` with a motion is deterministic and its render shows the
  posed body.
- A captured walk (synthetic, one step cycle a second) loops on a whole
  number of cycles without a jump; the arms of a T-pose capture hang
  along our body; the thighs swing as the captured ones; the same
  capture written Z-up, or walking in another direction, plays the
  same; a too short capture or a skeleton without a forearm is refused.
- A captured motion imported into a project drapes, is cached with its
  file and stale when the file changes; `motion` and `motion_file`
  together are refused; in the app, an imported motion is listed and
  plays.
- The library adds a file once, numbers another of the same name,
  renames (refusing names a file cannot have and names taken) and
  deletes; a file the avatar cannot play is not added. In the app, a
  library motion is in the Motion menu and plays from a copy in the
  project; the dialog deletes it, and the project keeps its copy.

## Provenance notes
The skeleton's joints and skin weights are MakeHuman's (CC0), reduced
to 24 bones by `cargo xtask fetch-makehuman`; nothing of MakeHuman's
code is used. The built-in motions are our own: feet placed and joint angles written
as smooth functions of the cycle after the published shapes and ranges
cited above, no motion capture data. Captured motions are
the user's files; none is shipped. The retargeting and the loop search
are our own (no code from motion capture tools).
