# Cloth physics

Status: living document (2026-11-14); the solver choice is revisable (see
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
| Vertex Block Descent [12] | Very promising (stable with large steps, GPU-friendly). It is a planned second backend behind the same trait once the MVP is solid. |
| IPC / C-IPC [13, 14] | Guaranteed intersection-free, but far from interactive at 10k vertices on a CPU. A possible offline "final quality" mode later. |
| GPU XPBD (wgpu compute) | Planned backend. CPU first because it is easier to debug and keep deterministic. |

### Solver trait

```rust
pub trait Solver: Send {
    fn rebuild(&mut self, scene: &SimScene, carry_over: Option<&PositionMap>);
    fn step(&mut self, frame_dt: f32, ctrl: &StepControl) -> StepStats;
    fn positions(&self) -> &[Vec3];
    fn set_material(&mut self, panel: PanelSimId, params: &FabricParams);
    fn set_pins(&mut self, pins: &[Pin]);
    fn set_grab(&mut self, grab: Option<GrabTarget>);
}
```

`SimScene` is plain data built by the sim bridge in `r3d-commands`:
particles (rest positions in 2D and 3D, mass), triangles, per-triangle
material frame (warp/weft directions from the grainline), seam pairs,
colliders (avatar SDF, ground plane) and material parameter sets.

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
| Avatar contact | Unilateral SDF constraint `φ(x) ≥ h` | [6], [7] | Thickness `h` from the fabric. |
| Self-contact | Particle–particle unilateral distance `≥ h`, excluding rest neighbors | [7], [8] | Basic self-collision for the MVP; see §5. |
| Friction | Position-based Coulomb friction (static/kinetic) on contacts | [7] | Separate cloth–body and cloth–cloth coefficients. |
| Damping | XPBD constraint damping plus light global velocity damping | [2] | Air drag later. |

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
| Cotton (poplin) | 120 | 0.30 | 8000 / 6000 | 400 | 3 % | 10 / 7 | 0.40 |
| Denim | 400 | 0.80 | 20000 / 15000 | 1500 | 2 % | 60 / 40 | 0.50 |
| Jersey (knit) | 180 | 0.60 | 300 / 150 | 100 | – | 4 / 3 | 0.50 |
| Silk (charmeuse) | 80 | 0.15 | 5000 / 4000 | 150 | 3 % | 1.5 / 1.2 | 0.25 |
| Wool (suiting) | 260 | 0.60 | 10000 / 8000 | 600 | 3 % | 20 / 15 | 0.45 |

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

### Avatar (static for the MVP)

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
- Later (animated avatars): a BVH over the moving mesh with continuous
  vertex–triangle tests [6].
- The cloth is kept at its thickness **plus a 2.5 mm skin margin** from the
  body: cloth triangles (~25 mm) and the avatar's are flat chords of curved
  surfaces, and without the margin the body shows through between cloth
  vertices.
- The margin alone did not hold across the bust: particles rested 3.4 mm
  out, but the middles of edges and triangles between them sank up to
  2 mm into the body, which showed through a tight dress or T-shirt. Mesh
  edges whose two particles touch the body are now sampled at their middle
  too, and both ends are pushed out (by inverse mass) so the middle keeps
  the same distance. Only touching edges are sampled, serially after the
  parallel particle pass (≈ 6 % more per step on the dress); the template
  tests check triangle middles as well as particles.
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


- **Real time with little headroom.** The three-piece skirt (1 700
  particles, 30 substeps) steps in about 14 ms per 16.7 ms frame on an
  Apple M-series laptop, most of it the serial constraint solve. Next
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

## 7. Performance targets

Reference machine: 8+ core desktop or laptop CPU (Apple M-series, recent x86).

| Scene | Particles | Target | Measured (2026-11-15, M-series, 30 substeps) |
|---|---|---|---|
| Skirt on avatar | ~4k | ≥ 60 fps, 15 substeps | 31 ms/frame (sheet on a sphere, `cargo bench -p r3d-sim`); the 1.7k-particle skirt: 14 ms |
| T-shirt on avatar | ~10k | ≥ 30 fps, 15 substeps (stretch: 60 fps) | 70 ms/frame |
| Dress | ~20k | ≥ 15 fps interactive; GPU backend later for 60 fps | — |

We are about 2× off: 30 substeps are needed for the membrane to converge
(§4), and the constraint solve is serial (§6). Leads: fewer substeps with a
better-converging solver (e.g. Vertex Block Descent [12]), or the GPU.

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

Reading order for contributors: [1] → [2] → [3] → [4] → [5] → [7] → [8] → [6].
