# Cloth physics

Status: living document (2026-11-19, M20); the solver choice is revisable (see
the alternatives below). All algorithms come from the
published papers listed in §9. Code that implements a method cites the paper
in its doc comment.

## 1. Choice for the MVP: XPBD with small steps, on the CPU

**Extended Position-Based Dynamics** [2] with the **"small steps"** scheme
[3]: many substeps per frame and a single constraint iteration per substep.

Why:

- **Robust**: it is unconditionally stable in practice and never explodes on
  bad input. That matters for a never-crash app where users drag points
  anywhere.
- **Stiffness does not depend on iteration count or time step** (it is set by
  the compliance), unlike classic PBD [1]. Fabric parameters therefore keep
  their meaning when we change substeps for performance.
- **Simple and well documented**: easy to test, to make deterministic and to
  parallelize with graph coloring.
- **Interactive**: mouse grab, pins and parameter edits are just constraints.

Alternatives considered:

| Method | Why not now |
|---|---|
| Implicit Euler / Baraff–Witkin [10] | Needs a linear solve every step. Damping artifacts. Harder to make robust with contact. |
| Projective Dynamics [11] | Fast with a prefactorized system, but refactorization is needed whenever the topology or pins change (frequent while editing). |
| Vertex Block Descent [12] | Very promising (stable with large steps, GPU-friendly). Not needed for M17: on the GPU, XPBD's cost is per dispatch, not per particle (§7), so VBD would pay off only by cutting substeps. The lead when later milestones need more. |
| IPC / C-IPC [13, 14] | Guaranteed intersection-free, but far from interactive at 10k vertices on a CPU. A possible offline "final quality" mode later. |
| GPU XPBD (wgpu compute) | Done (M17, `r3d-sim-gpu`, §6): the app's backend. The CPU stays the reference, deterministic for any thread count. |

### Solver trait

```rust
pub trait Solver: Send + Debug {
    fn backend(&self) -> &'static str;               // "cpu" or "gpu"
    fn step(&mut self, frame_dt: f32) -> Result<(), StepError>;
    fn positions(&self) -> &[Vec3];
    fn time(&self) -> f32;
    fn stats(&self) -> StepStats;
    fn continue_at(&mut self, t: f32);               // after a rebuild
    fn grab(&mut self, index: usize);
    fn move_grab(&mut self, target: Vec3);
    fn release(&mut self);
}
```

`SimScene` is plain data built by the sim bridge in `r3d-commands`:
particles (rest positions in 2D and 3D, mass), triangles, per-triangle
material frame (warp/weft directions from the grainline), seam pairs,
colliders (avatar SDF, ground plane) and material parameter sets.
`Model::new` compiles it once into what every backend runs: per-particle
data, flat constraint arrays with their compliances, the self-contact
topology. A scene edit rebuilds the solver (pins and materials are part
of the scene); the cloth carries over (`drape::carry_over`). A step
returns an error only when a GPU loses its device.

## 2. Discretization

- Each panel is triangulated in 2D (constrained Delaunay, `r3d-geom`) with a
  target **particle spacing** (default 10 mm, so ~10k particles for a
  T-shirt). Spacing can be set per panel.
- Seam edges are resampled so that both sides of a seam get the **same
  number of vertices**, distributed proportionally to arc length. This makes
  seam pairing exact.
- The 2D rest shape gives the rest metric for every constraint, so the cloth
  "wants" to become its pattern shape, which is what makes patterns behave
  like real panels.
- Particle mass = fabric density × the area of the Voronoi region (one third
  of the adjacent triangle areas).

## 3. Constraints and fabric model

All constraints are XPBD constraints with compliance `α = 1/k`, scaled by the
element's rest area or length as each method prescribes.

| Effect | Constraint | Reference | Notes |
|---|---|---|---|
| Stretch warp/weft, shear | Triangle **strain-based** constraint in the material frame (warp = u, weft = v) | Strain Based Dynamics [4] | Gives three independent stiffnesses: warp, weft, shear. Essential for woven vs knit behavior. |
| Stretch limit (wovens) | Unilateral strain limit (inequality constraint) | [4], [15] | Woven fabrics are nearly inextensible past a small strain. Keeps the main stiffness low enough for good wrinkles. |
| Bending | **Isometric quadratic bending** on interior edges (4-point stencil) | Bergou et al. [5] | Constant Hessian, cheap and stable. Anisotropy approximated by weighting each hinge by the angle between the edge and the grain. Dihedral bending [1] is the fallback. |
| Seams | Zero-rest-length distance constraints between paired seam vertices, compliance ramped from soft to stiff during "sewing" | [2] | When a seam gap goes under a threshold, the pair can be **welded** into one particle (more stable, fewer constraints). |
| Pins / grab | Attachment constraint to a world target, with its own compliance | [2] | The mouse grab uses a soft compliance, so cloth follows the cursor without exploding. |
| Closures (buttons, zippers) | Stiff distance constraint whose rest length shrinks to zero at a bounded speed | [2] | See "Closures" below: fastened at any time without snapping. |
| Avatar contact | Unilateral SDF constraint `φ(x) ≥ h` | [6], [7] | Thickness `h` from the fabric. |
| Self-contact | Particle–particle unilateral distance `≥ h`, excluding rest neighbors | [7], [8] | Basic self-collision for the MVP; see §5. |
| Friction | Position-based Coulomb friction (static/kinetic) on contacts | [7] | Separate cloth–body and cloth–cloth coefficients. |
| Damping | XPBD constraint damping plus light global velocity damping | [2] | Air drag later. |

### Closures (M20)

A buttoned button holds two points of the garment together (the button
and the buttonhole's end, both made mesh vertices when they are clear of
the outline, else the nearest vertex); a closed zipper holds its two
sides point for point like a seam, from its bottom stop up to the slider
(the stop always holds). Seams close on the sewing ramp (§4), which is
over after 0.6 s: a button fastened later at full stiffness would pull
two points a few centimetres apart together in one substep (≈ 50 m/s).
A closure is therefore an XPBD distance constraint [2] with the
compliance of a closed seam (10⁻⁹ m/N) whose target is
`max(0, d₀ − v·h)`, `d₀` the pair's distance at the substep's start and
`v` = 0.5 m/s: the gap shrinks at that speed, held both ways so the
cloth's own momentum cannot close it faster (limiting only the
correction let the momentum build: a 3 cm gap shut in one frame at
2.8 m/s). Once closed it is a stiff zero-length tie.

- Opening or closing a trim is a document edit: the app rebuilds the
  scene and carries the cloth over (§7.8 of the architecture), so the
  drape goes on; the closures are the only constraints that change.
- A button holds two layers on top of each other: cloth within 6 cm of
  the button and cloth within 6 cm of its buttonhole (along the cloth)
  never repel (self-collision would keep a front band 2 cm off the one
  under it). Closures join the cloth graph for self-collision as seams
  do (neighbours excluded, closed pairs pushed as one).
- On the GPU, closures are coloured like seams, one dispatch per colour
  after the seams.
- Checked on both backends (`buttons_hold_two_bands_together`): two
  bands 3 cm apart held by three buttons close at the bounded speed
  (gap still > 2 cm after one frame, speed < 2 m/s), are closed within
  2 mm after 1.5 s and hang together; unbuttoned they hang apart. A
  skirt with a centre-back zipper (`r3d-commands` `tests/closures.rs`)
  drapes zipped, opens (only the stop holds), and closes again within
  2 mm in a second, each time carried over without starting again.

### Fabric parameters (our own documented model)

| Parameter | Symbol | Unit | Meaning |
|---|---|---|---|
| Density | ρ | g/m² | Mass per area |
| Thickness | h | mm | Collision thickness, both sides |
| Stretch, warp | k_u | N/m | In-plane stiffness along the grainline |
| Stretch, weft | k_v | N/m | In-plane stiffness across the grainline |
| Shear | k_s | N/m | In-plane shear stiffness |
| Stretch limit | ε_max | % | Strain where the fabric locks up (optional, wovens) |
| Bending, warp | B_u | µN·m | Bending rigidity for folds across the warp |
| Bending, weft | B_v | µN·m | Bending rigidity for folds across the weft |
| Friction (body) | μ_b | – | Cloth–avatar Coulomb coefficient |
| Friction (self) | μ_c | – | Cloth–cloth coefficient |
| Damping | d | 0–1 | Normalized internal damping |

The UI shows these in plain words ("Stretch", "Stiffness", "Weight") with
sliders that map to the physical values on a log scale. The raw values are
in the "Advanced" fold.

### Presets (initial estimates, to be calibrated)

Values are order-of-magnitude starting points from general textile knowledge
(fabric weights, typical bending rigidity ranges from the textile-testing
literature [16]). They will be tuned on reference scenes, and later with the
measurement-based calibration on the roadmap.

| Preset | ρ g/m² | h mm | k_u / k_v N/m | k_s N/m | ε_max | B_u / B_v µN·m | μ_b |
|---|---|---|---|---|---|---|---|
| Cotton (poplin) | 120 | 0.30 | 8000 / 6000 | 40 | 3 % | 30 / 21 | 0.40 |
| Denim | 400 | 0.80 | 20000 / 15000 | 150 | 2 % | 60 / 40 | 0.50 |
| Jersey (knit) | 180 | 0.60 | 300 / 150 | 10 | – | 4 / 3 | 0.50 |
| Silk (charmeuse) | 80 | 0.15 | 5000 / 4000 | 15 | 3 % | 1.5 / 1.2 | 0.25 |
| Wool (suiting) | 260 | 0.60 | 10000 / 8000 | 60 | 3 % | 20 / 15 | 0.45 |

Shear k_s is an order of magnitude below tension, as measured on wovens
(Kawabata shear rigidity ≈ 0.5–2 gf/cm/° ≈ 30–110 N/m for shirtings and
suitings [16]). The first presets had it at 1/20–1/7 of tension: the
linear triangles, kept from shearing, could not bend smoothly between
vertices and the back of a dress set in crumples a few triangles wide
(mean dihedral angle 8.7° against 4.5° now). Cotton's bending was raised
from 10 to 30 µN·m (a firm cotton): at 25 mm spacing the mesh cannot
show folds finer than a few centimetres anyway. Measured on the dress
after 4 s; the skirt still holds at the waist.

## 4. Time integration

- Fixed frame `dt = 1/60 s`, **N substeps** (30) with 1 iteration each [3].
  Fewer substeps leave the membrane unconverged on garment-sized meshes: at
  15, a skirt fitted at the waist slowly creeps down to the hips.
- Per substep: predict positions (gravity, external forces) → solve
  constraints in fixed color order → update velocities → apply friction and
  damping.
- Sewing phase: seam compliance ramps from soft to stiff over 0.6 s of
  simulated time, and in a scene with seams gravity ramps from zero to
  full over the same time, so that panels do not fall before they are
  sewn (a skirt that closed below the waist would slide down the hips).
- Simulated time is decoupled from wall time. If a frame is late, we drop
  simulated time rather than take larger steps.

## 5. Collisions

### Avatar

- When the avatar is fitted, build a **narrow-band signed distance field**
  (`collide::StaticMesh`): nodes 5 mm apart, computed only within 2 cm of
  the surface and stored in 8×8×8 bricks (a few MB). The sign comes from
  angle-weighted pseudo-normals [17], exact for closed
  manifold meshes; the MakeHuman body is closed (checked: no boundary or
  non-manifold edge). Built in parallel per brick in ~0.16 s, so it is not
  cached on disk. (The winding-number sign [9] stays the plan for open or
  imported avatars.)
- Particle vs SDF at every substep: trilinear distance and gradient,
  constant time. Penetrations up to the band (2 cm) are pushed back out,
  far more than a particle moves in a substep.
- A moving avatar (M23) keeps this field, built once in its rest pose;
  see "Moving body" below.
- The cloth is kept at its thickness **plus a 2.5 mm skin margin** from the
  body: cloth triangles (~25 mm) and the avatar's are flat chords of curved
  surfaces, and without the margin the body shows through between cloth
  vertices.
- The margin alone did not hold across the bust: particles rested 3.4 mm
  out, but the middles of edges and triangles between them sank up to
  2 mm into the body, which showed through a tight dress or T-shirt. Mesh
  edges whose two particles touch the body are now sampled at their middle
  too, and both ends are pushed out (by inverse mass) so the middle keeps
  the same distance. Only edges whose ends are near the body (within
  their contact distance + 5 mm) are sampled, serially after the
  parallel particle pass (≈ 6 % more per step on the dress); the template
  tests check triangle middles as well as particles.
- Those edges first required both ends *touching* the body. Pushing the
  ends out took them off the skin, so the edge let go, they fell back
  and were pushed again: every substep, a push turned into velocity and
  the garment never came to rest (a T-shirt fluttered as if in a draught,
  12 cm/s on average for ever). The 5 mm band keeps an edge holding the
  ends it pushed out, and a particle's pushes from its ~6 edges are
  averaged (constraint averaging [7] §4.2), not summed. Settled cloth now
  moves ≈ 1 mm/s; a template test checks the T-shirt comes to rest.
- **Friction** against the body is Coulomb friction in positions [7]: the
  sliding of a contact in a substep is cancelled up to μ_b × the depth the
  particle was pushed out (its normal force), so cloth holds where it
  presses and slides where it only rests. It used to cancel a fixed
  fraction μ_b of the sliding every substep: over 30 substeps that is
  0.7³⁰ ≈ 2·10⁻⁵ of it left per frame, so cloth touching the body was
  glued there and folds made while the garment closed never slid out (the
  back of the dress stayed crumpled; seen from the back only).
- Without self-collision (below) a wide hem can fold through itself; this
  is visible on the A-line skirt and is the next physics item (M7).

### Moving body (M23)

The avatar moves by its skeleton (24 bones); its skin follows by linear
blend skinning [19], each vertex by up to four bones. Rebuilding the
distance field of the posed body every frame (0.16 s on the CPU) is too
slow, and a BVH over the moving mesh with continuous tests [6] would
replace the constant-time query the solver is built on. Instead the
rest pose's field is queried through the skinning (our own scheme,
`r3d_sim::motion`):

- The field also stores, per node, the body vertex nearest to its
  closest surface point, and so its skin weights; and, per bone, the box
  of the nodes whose vertex it leads (weighs at least half the vertex's
  heaviest bone).
- **Inverse skinning.** To see the body from a particle `p`, find the
  rest point `q` whose skinned position `Σ wₖ Gₖ(q)` is `p`. With the
  weights held constant near `q` it is one linear solve,
  `q = M⁻¹ (p − T)` with `M = Σ wₖ Rₖ` and `T = Σ wₖ tₖ`, repeated three
  times with the weights found at the new `q`. Each bone whose rest box
  holds `Gᵦ⁻¹(p)` and leads the skin there gives a starting point, so
  every limb near `p` is tried; a start is kept only if the skin it
  finds passes within 2 mm of `p`. The distance read at `q` is mapped to
  the posed skin by `M` to first order (`d / |M⁻ᵀ n|`, along `M⁻ᵀ n`),
  and of the points found the deepest wins. The cloth thus collides with
  the very skin the views draw.
- A first version took the body as the union of its bones' rigid pieces
  (each bone's inverse motion only). Raising an arm swept the rigid copy
  of the shoulder and the chest beside the armpit up through the shirt,
  where the blended skin does not go: deep false contacts that threw the
  cloth at 15 m/s. Following the blended skin removed them.
- **Pre-stabilisation** (as in [7]): a contact gives the cloth no more speed
  along the normal than the surface touched has; the rest of the push (a
  penetration the cloth already had) moves the particle's previous
  position too, so it is undone without throwing the cloth out. Only
  for a moving body: a still one drapes exactly as before (trousers held
  at the hips by friction slid down with it). Where
  linear blend skinning folds the skin onto itself (an arm raised high
  folds the armpit), cloth caught in the fold was squeezed out at
  10–20 m/s; the arms are therefore raised to about 40° above the
  horizontal, which MakeHuman's weights (made for the A pose) handle.
- **Friction** cancels the sliding relative to the touched surface,
  which moved by `p − (M_before q + T_before)` during the substep: the
  cloth is carried along with the body.
- The motion is sampled 60 times a second and interpolated per substep
  (rotations by slerp); the GPU gets it once per frame (per substep and
  bone: the motion at its end, its inverse and the motion at its start).
  Cost: about 24 inverse transforms and box tests per particle and
  substep, then a few inverse-skinning solves; the dress walks at
  ≈ 80 ms per frame on the CPU (debug-optimised build), and well within
  the frame on the GPU.
- Checked on both backends: a sheet lying on a box that moves 20 cm
  sideways and 5 cm up in a second goes along within 3 cm and never goes
  inside it (`a_moving_body_carries_the_cloth`); a dress walking, a
  skirt turning and a T-shirt raising its arms stay out of the moving
  skin by less than 3 mm with their seams closed (`r3d-commands`
  `tests/motion.rs`, `r3d-sim-gpu` `a_dress_walks_on_the_gpu`).

### Starting positions

Placed panels start wrapped around the body's vertical axis
(`r3d_commands::drape`): the panel's centre at its placement angle, its top
at the placement height, horizontal pattern distances kept as arc lengths.
At each height the radius is the larger of
- how far out the body reaches at that angle, anywhere between the panel's
  top and that height (a garment starts outside everything above it), plus
  the placement offset. The reach is tabulated per 1 cm of height and 10°
  of angle from the body's vertices (arms, farther than 0.3 m from the
  axis, left out), gaps filled over ±2 cm / ±30° and smoothed; an earlier
  girth / 2π circle let the hips show through at the sides;
- the garment's own circumference there (the widths of all placed panels
  side by side) / 2π, so the panels start just touching.

Arc lengths along the pattern are kept at the chosen radius (the angle and
the radius are settled by two fixed-point steps), so the start pose is not
stretched: an inconsistent first version stretched the panels at the hips
and the hem bunched up 7 cm when they relaxed.

An earlier fixed flare (radius growing 0.35 m per metre) started an A-line
hem at 2.5 m around for 1.56 m of cloth: the seams pulled it in and the hem
buckled inward. With the garment's own width it hangs as an A-line.

Where a seam must close a gap at the start, it pulls the cloth straight
across the gap, inside the circle. The skirts placed their two backs by
their middles at 120° and 240°, as if the three pieces were equal: the
front is half the hem, so a 30° gap opened at the centre back, and the
seam closed it between the legs, where the back caught in a fold. The
fold went unnoticed while the cloth kept fluttering (§5): the flutter
shook it partly loose.

- The skirt templates now place their backs by their centre-back edges,
  side by side at 180° (`centre_x_mm`): the seam over the gap between the
  legs starts closed, and the gaps the flare leaves at the waist close
  at the side seams, over the hips. The back hangs free.
- `garment.arrange` places pieces side by side, each taking a share of
  the turn in proportion to its width (backs at 135° and 225°: centre
  backs meeting at the hem). Higher up the body's reach sets a larger
  radius, so they overlap a little there rather than gap; the waist
  starts looser than the templates' and a slight inward dip remains at
  the centre-back hem. Placing each piece by its seamed edges in general
  is left for later.

Sleeves and trouser legs (M18). Around an arm a panel starts as a closed
tube at its own width / 2π, and the collisions push it out where the arm
is thicker. Starting outside the arm's reach (plus the placement offset,
or even 5 mm) opened a 23 cm cuff into a 145° part of a tube, and a
men's sleeve 15 cm wide at the elbow; the seams closed them across the
arm in a few frames and caught a fold on one arm. The reach (the
farthest arm point from an axis not quite centred) overstates the arm.
The forearm's axis now aims from the elbow at the middle of the wrist,
the thinnest part of the forearm: the principal axis of the short band
below the elbow was off by tens of degrees. Around a leg (`left_leg`, `right_leg`) a trouser piece
starts:
- below the crotch, as a sleeve around the leg's axis: the line through
  the middles of the thigh (0.34 of stature) and the shin (0.10), slices'
  bounding boxes. A principal axis of a thigh band tilted the legs: a
  male body's genitals reach down to 0.40 of stature, and a shorter band
  was too squat to have one. The reach is the body's on that side near
  the piece's own height (±3 cm), not everything above it as for a skirt;
  the hips then started the legs twice as wide as they are cut;
- above the crotch (the lowest body point on the median plane), around
  the body: the piece's centre front or back edge at 0° or 180°, the
  front and back of a leg sharing the half turn in proportion to their
  widths so the side seams meet. Wrapping the whole piece around its
  leg's axis there covered half a ring: the side seams started 10–40 cm
  apart and pulled the waist down to the hips on a male body;
- blended over 8 cm above the crotch; cloth that would cross the median
  plane is pressed flat 6 mm from it, in front of (or behind) the body.
Trousers hold at the waist by friction alone (no ease there): the CPU and
GPU contact solvers settle them about 3 cm apart, both at rest.

### Several garments (M19)

Each garment has its fabric (one `FabricParams` per garment, triangles
indexed by their piece's garment). Garments start in layer order, each
`LAYER_GAP_M` (25 mm) further out per garment under it; with several
garments, pieces around the body start just outside it (5 mm) near their
own height, not at their placement offset outside everything above them:
both open the side seams (12–22 cm on the shirt), and a seam closing
across a gap cuts straight through the garment underneath.

Close layers do not work yet. Self-collision keeps particles two radii
apart (§5: 0.4 of the spacing each, 20 mm at 25 mm): two layers 8 mm
apart were pushed past each other at once, and fitted trousers cannot
leave 2 cm for a shirt under their waist. Tried for a shirt tucked into
trousers, all measured on the first frame:
- the trousers started around the shirt, spread around the larger circle
  (stretched 20–47 %): they snapped back through it;
- the trousers kept their widths (side seams apart): the seams closed
  straight across, through the shirt;
- the shirt started gathered inside the trousers, at their natural size,
  with compression made 100 to 10⁴ times softer than stretch: the shirt
  was still pushed out within a frame (the softer compression made no
  difference and was not kept).
Worn over the trousers, the shirt drapes with closed seams but the two
show through each other where they touch. The way forward is thin
cloth–cloth contact (vertex–triangle, with a thickness of millimetres)
between garments, in both solvers.

Below the crotch, cloth pressed against the median plane stays where it
is (nothing lies between the legs): pushed in front of or behind the body
as above the crotch, the back crotch point started 18 cm behind the front
one and the inseam swept through the gap.

### Self-collision

- **Current** (`selfcollide`): particle–particle repulsion [7]. Each
  particle is a sphere of radius 0.4 × the median rest edge length (≈ 10 mm
  at 25 mm spacing), so one layer cannot slip between another's particles.
  Pairs closer than 4 radii *along the cloth* (bounded Dijkstra over mesh
  edges and seams, at build time) never collide: they touch because the
  cloth folds, and repelling them would stiffen ordinary folds. Candidate
  pairs come from a spatial hash [8] once per frame (with a 1 cm margin
  for motion) and are resolved every substep, sorted for determinism.
  Particles sewn together (seam groups, by union–find) that have closed to
  within a radius move as one particle of their summed mass when pushed:
  contacts are solved after the seams, and pushing one side alone held a
  seam open by ≈ 2 mm where a fold of the cloth lay on it.
- Consequences: layers stay apart by up to one particle diameter (less
  where particles nest in each other's gaps, ≈ 8 mm on a regular grid), so
  folds look slightly puffy; it costs ≈ 1 ms per frame on the skirt.
- **Later**: vertex–triangle and edge–edge continuous collision [6] for
  thin layers, and untangling for already-intersecting states.

### Known issues


- **Real time with little headroom on the CPU.** The three-piece skirt
  (1 700 particles, 30 substeps) steps in about 14 ms per 16.7 ms frame
  on an Apple M-series laptop, most of it the serial constraint solve.
  The app now drapes on the GPU (§6, §7). Next
  speed-up: fewer substeps once convergence improves. With damping set to
  0, kinetic + gravitational energy decreases steadily: the solver is
  stable; a sheet hung by two corners needs about ten seconds to come to
  rest.
- **Fabric presets are first estimates** (§3). Cotton's bending length
  (≈ 2 cm) is plausible; calibrate all presets against simple physical
  tests (cantilever bending length, drape coefficient) when comparing
  garments with real ones.

## 6. Parallelism and determinism

- **Contacts** are independent per particle and run in parallel with
  rayon: one dispatch per substep, identical results for any thread count
  (tested). (Before the SDF they were a closest-triangle search and 60 % of
  a step.)
- **Constraints** are solved serially (Gauss–Seidel, fixed order). Splitting
  them into independent sets by graph colouring and solving each colour in
  parallel was measured on the skirt (1 700 particles, 11 membrane + 20
  hinge colours, 30 substeps): ≈ 1 000 parallel dispatches per frame of a
  few tens of µs each made it twice slower than serial (45 vs 22 ms/frame).
  It becomes worth revisiting for much larger meshes, or with one dispatch
  per substep over a whole colour set (e.g. on the GPU).
- **Deterministic**: fixed substeps and constraint order, no reductions
  whose order depends on thread scheduling.
- Memory layout: struct-of-arrays (`Vec<Vec3>` for positions, previous
  positions, velocities, inverse masses), constraints in flat arrays per
  type.

### On the GPU (`r3d-sim-gpu`, M17)

The same model and substep on wgpu compute, one WGSL module with a
kernel per stage:

- **Constraints in colours.** Membranes, hinges and seams are each split
  by greedy graph colouring into sets that share no particle (≈ 7, 15
  and 3 colours on a garment); a colour is one dispatch, so the sweep is
  still Gauss–Seidel, in colour order instead of index order. The ≈ 1 000
  dispatches per frame that made per-colour parallelism twice slower on
  the CPU (above) cost ≈ 4 µs each on a GPU.
- **Contacts gather.** Body contacts, the edges kept out of the body and
  self-collision are computed per particle from what it reads (Jacobi),
  then applied, so no two invocations write one particle; no atomics on
  positions.
- **Self-collision.** Candidates come from a spatial hash rebuilt on the
  GPU once per frame [8] (count, prefix sum, scatter), each particle's
  list kept sorted by index (32 at most), so a run is deterministic on a
  given GPU; they are the CPU's pairs exactly (checked). The pushes are
  gathered and a pair is scaled by 1 / the larger contact count of its
  two particles, the same on both sides. Averaging each particle by its
  own count (constraint averaging [7] §4.2) moved the two sides of a pair
  by unequal impulses: tangled cloth propelled itself (a sheet hung from
  a held corner whipped at 10 m/s, against 2 m/s on the CPU). Particles
  sewn together and closed move as one, as on the CPU: each adds the
  pushes of its welded partners.
- **Per frame**: one command buffer (the substeps' uniforms in 256-byte
  slots, dynamic offsets), then positions and velocities read back for
  the views and the statistics. Device or validation errors become a
  `StepError`; the app then carries on from the last positions on the
  CPU.
- **Rebuilds** (every 100 ms while a pattern edit is dragged during a
  drape): the kernels are compiled once per device (`GpuContext`, about
  0.3 s the first time, 25 ms once the driver has cached them) and the
  body's distance field (~7 MB) is uploaded once while it is the same;
  a new solver then costs ≈ 5 ms on top of the model's 5 ms.
- **Where it runs**: the app on its one shared device; headless
  `simulate` (CLI, MCP) on the CPU, so a cached drape is reproducible.

## 7. Performance targets

Reference machine: 8+ core desktop or laptop CPU (Apple M-series, recent x86).

| Scene | Particles | Target | Measured (2026-11-15, M-series, 30 substeps) |
|---|---|---|---|
| Skirt on avatar | ~4k | ≥ 60 fps, 15 substeps | 31 ms/frame (sheet on a sphere, `cargo bench -p r3d-sim`); the 1.7k-particle skirt: 14 ms |
| T-shirt on avatar | ~10k | ≥ 30 fps, 15 substeps (stretch: 60 fps) | 70 ms/frame |
| Dress | ~20k | ≥ 15 fps interactive; GPU backend later for 60 fps | — |

The CPU was about 2× off: 30 substeps are needed for the membrane to
converge (§4), and the constraint solve is serial (§6).

M17 targets the starter garments at the default 25 mm spacing:
T-shirt ≥ 60 fps, dress ≥ 30 fps. `cargo bench -p r3d-sim-gpu`
(2026-11-17, RTX 2080 + 16-thread x86 CPU, 30 substeps, after 1 s of
draping):

| Garment | Particles | CPU | GPU |
|---|---|---|---|
| T-shirt | 1 776 | 29.7 ms/frame | 5.4 ms/frame |
| Dress | 2 264 | 36.7 ms/frame | 5.3 ms/frame |

In the app, sharing the GPU with the views, the T-shirt steps in 7.8 ms
and the views keep 60 fps. The GPU cost is per dispatch (≈ 42 per
substep), not per particle: the dress meshed at 2.3k, 4.4k and 6.5k
particles steps in 5.4, 5.7 and 6.2 ms. Fewer substeps (VBD [12]) or
fewer colours (one kernel over all constraint kinds) are the next levers.
Not measured yet: Apple M-series (Metal), Linux (Vulkan).

Budget per 30 fps frame: ≤ 20 ms solver, ≤ 3 ms rebuild of the spatial hash,
with the rest left for rendering and UI. Rebuilding the scene after a pattern
edit takes ≤ 50 ms for a T-shirt, so editing stays fluid. `criterion`
benchmarks track these numbers per release.

## 8. Validation scenes (regression tests)

1. Hanging square pinned at two corners. Checks: sag within range of the
   material, no NaN, energy decays.
2. Cloth dropped on a sphere. Checks: no penetration beyond `h/2`, rest state
   reached.
3. Two panels sewn into a tube. Checks: seam gap < 1 mm after the sewing phase.
4. Skirt on the default avatar. Checks: no penetration, hem height stable,
   frame time within budget (benchmark).
5. Anisotropy check: the same panel hung by its warp vs its weft gives
   different elongation, in the right order.

The scenes and their checks live in `r3d-sim/tests/reference` and run on
both backends (the GPU's tests include the module), with stacked sheets
and a grabbed corner. `r3d-sim-gpu/tests/garments.rs` drapes every
starter garment on both and checks the template criteria (nothing inside
the body, triangles not cut into it, seams closed, the T-shirt at rest)
and that the GPU drape ends within 2 cm of the CPU's (it is within a few
mm).

### Reading the fit (M24)

`r3d_commands::fit` (spec 24) measures a drape after the fact, from the
positions alone, so the CPU and GPU results read alike:
- **Strain** of a triangle: the largest principal stretch of its
  deformation gradient `F` (rest shape from the pattern, `FᵀF`'s larger
  eigenvalue, square root) minus one [20], as Strain Based Dynamics
  writes the same `F` [4]. Not the solver's constraint error: a jersey
  T-shirt after 5 s reads 1–6 % on average per piece and up to 40 %
  under the arms, where the arms in their A pose pull the sleeves; a
  woven stays near its stretch limit (≈ 3 %).
- **Gap**: each vertex's distance to the body's triangles (closest
  point on a triangle [18], triangles bucketed in a 4 cm grid). Cloth
  that touches reads 3–5 mm (thickness and the skin margin, §5).
- **Tape**: a horizontal cut through the cloth's triangles, joined
  across seams (the seam pairs name one vertex for both sides) and
  across gaps under 2 cm; the body's cut is measured by its convex hull,
  as a tape around the body reads it (`r3d_avatar::measure`).

## 9. References

1. M. Müller, B. Heidelberger, M. Hennix, J. Ratcliff. *Position Based Dynamics.* VRIPHYS 2006 / JVCIR 2007.
2. M. Macklin, M. Müller, N. Chentanez. *XPBD: Position-Based Simulation of Compliant Constrained Dynamics.* MIG 2016.
3. M. Macklin, K. Storey, M. Lu, P. Terdiman, N. Chentanez, S. Jeschke, M. Müller. *Small Steps in Physics Simulation.* SCA 2019.
4. M. Müller, N. Chentanez, T.-Y. Kim, M. Macklin. *Strain Based Dynamics.* SCA 2014.
5. M. Bergou, M. Wardetzky, D. Harmon, D. Zorin, E. Grinspun. *A Quadratic Bending Model for Inextensible Surfaces.* SGP 2006.
6. R. Bridson, R. Fedkiw, J. Anderson. *Robust Treatment of Collisions, Contact and Friction for Cloth Animation.* SIGGRAPH 2002.
7. M. Macklin, M. Müller, N. Chentanez, T.-Y. Kim. *Unified Particle Physics for Real-Time Applications.* SIGGRAPH 2014.
8. M. Teschner, B. Heidelberger, M. Müller, D. Pomeranets, M. Gross. *Optimized Spatial Hashing for Collision Detection of Deformable Objects.* VMV 2003.
9. G. Barill, N. Dickson, R. Schmidt, D. Levin, A. Jacobson. *Fast Winding Numbers for Soups and Clouds.* SIGGRAPH 2018.
10. D. Baraff, A. Witkin. *Large Steps in Cloth Simulation.* SIGGRAPH 1998.
11. S. Bouaziz, S. Martin, T. Liu, L. Kavan, M. Pauly. *Projective Dynamics: Fusing Constraint Projections for Fast Simulation.* SIGGRAPH 2014.
12. A. H. Chen, Z. Liu, Y. Yang, C. Yuksel. *Vertex Block Descent.* SIGGRAPH 2024.
13. M. Li, Z. Ferguson, T. Schneider, T. Langlois, D. Zorin, D. Panozzo, C. Jiang, D. Kaufman. *Incremental Potential Contact.* SIGGRAPH 2020.
14. M. Li, D. Kaufman, C. Jiang. *Codimensional Incremental Potential Contact.* SIGGRAPH 2021.
15. X. Provot. *Deformation Constraints in a Mass-Spring Model to Describe Rigid Cloth Behavior.* Graphics Interface 1995.
16. S. Kawabata. *The Standardization and Analysis of Hand Evaluation.* Textile Machinery Society of Japan, 1980.
17. J. A. Bærentzen, H. Aanæs. *Signed Distance Computation Using the Angle Weighted Pseudonormal.* IEEE TVCG 11(3), 2005.
18. C. Ericson. *Real-Time Collision Detection*, §5.1.5 (closest point on a triangle). Morgan Kaufmann, 2004.
19. N. Magnenat-Thalmann, R. Laperrière, D. Thalmann. *Joint-Dependent Local Deformations for Hand Animation and Object Grasping.* Graphics Interface 1988.
20. J. Bonet, R. D. Wood. *Nonlinear Continuum Mechanics for Finite Element Analysis.* Cambridge University Press, 1997 (§3: deformation gradient, principal stretches).

Reading order for contributors: [1] → [2] → [3] → [4] → [5] → [7] → [8] → [6].
