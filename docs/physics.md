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
- Sewing phase: seam compliance ramps from soft to stiff over ~0.5 s of
  simulated time, and gravity can optionally be reduced while seams close,
  so that panels do not fall before they are sewn.
- Simulated time is decoupled from wall time. If a frame is late, we drop
  simulated time rather than take larger steps.

## 5. Collisions

### Avatar (static for the MVP)

- When the avatar is loaded, build a **signed distance field** on a regular
  grid (default 4 mm voxels, narrow band). The sign comes from the
  generalized winding number [9], which is robust to small holes in the
  mesh. It is built in the background and cached in `.r3d-cache/`.
- Particle vs SDF at every substep, with thickness `h`. Small substeps keep
  per-substep displacement below `h` at normal speeds, which prevents
  tunneling. Velocity clamping per substep is a safety net.
- Later (animated avatars): a BVH over the moving mesh with continuous
  vertex–triangle tests [6].
- **Current implementation** (until the SDF lands): closest point on the
  avatar's triangles through a uniform grid (`collide::StaticMesh`). The
  search radius is thickness + 5 mm + the distance the particle moved in
  the substep: a particle outside at the start of the substep cannot be
  deeper than that, so fast motion (seams pulling a tight waist in) does
  not tunnel through the body.

### Starting positions

Placed panels start wrapped around the body's vertical axis
(`r3d_commands::drape`): the panel's centre at its placement angle, its top
at the placement height, horizontal pattern distances kept as arc lengths.
The radius at each height is the widest body section between the panel's
top and that height (girth / 2π, so a garment starts outside everything
above it), plus the placement offset, plus a flare of 0.35 m per metre so
that flared pieces do not overlap before the seams close.

### Self-collision

- **MVP**: particle–particle repulsion with a uniform **spatial hash** [8]
  rebuilt every substep (or every few substeps), radius = `h`, skipping
  pairs that are close in rest topology.
- **Later**: vertex–triangle and edge–edge continuous collision [6], and
  untangling for already-intersecting states.

### Known issues

- **Not yet real time for a whole garment.** At 30 substeps a step of the
  three-piece skirt (1 700 particles) takes about 20 ms on one core, so the
  simulation runs slightly slower than real time (late frames drop
  simulated time). Next: rayon with graph colouring (§6). With damping set
  to 0, kinetic + gravitational energy decreases steadily (checked at 15
  and 60 substeps): the solver is stable; a sheet hung by two corners just
  needs about ten seconds to come to rest.

## 6. Parallelism and determinism

- Constraints are split into **independent sets by graph coloring** at
  rebuild. Each color is solved in parallel with rayon (Gauss–Seidel across
  colors, no write conflicts inside a color).
- Contacts are solved Jacobi-style with averaged corrections, in a fixed
  order.
- **Deterministic mode** (tests, CLI `--deterministic`): fixed substeps, fixed
  color order, no floating-point reductions whose order depends on thread
  scheduling, and identical results for any thread count. Regression tests
  compare positions against stored references with a tolerance.
- Memory layout: struct-of-arrays (`Vec<Vec3>` for positions, predicted
  positions, velocities, inverse masses), constraints in flat arrays per
  type, so that the inner loops can be auto-vectorized.

## 7. Performance targets

Reference machine: 8+ core desktop or laptop CPU (Apple M-series, recent x86).

| Scene | Particles | Target |
|---|---|---|
| Skirt on avatar | ~4k | ≥ 60 fps, 15 substeps |
| T-shirt on avatar | ~10k | ≥ 30 fps, 15 substeps (stretch: 60 fps) |
| Dress | ~20k | ≥ 15 fps interactive; GPU backend later for 60 fps |

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

Reading order for contributors: [1] → [2] → [3] → [4] → [5] → [7] → [8] → [6].
