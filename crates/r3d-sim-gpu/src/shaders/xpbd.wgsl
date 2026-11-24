// XPBD with small steps on the GPU (Macklin et al. 2016, 2019): the same
// constraints as the CPU backend (r3d-sim's `cpu.rs`, docs/physics.md §3),
// one kernel per step of a substep. Constraints of a kind are split into
// colours (no two in a colour share a particle), so each colour is one
// parallel dispatch of a Gauss–Seidel sweep. Contacts with the body, edge
// contacts and self-collision are per particle and gather what they need,
// so no two invocations write the same particle.
//
// Prepended by the Rust side: SKIN, NEAR, BAND, BRICK, MAX_NEIGHBOURS,
// ITERATIONS, ON_SKIN, CLOSURE_COMPLIANCE, CLOSING_SPEED, MAX_TOUCHES,
// LAYER_SKIN, OVERHANG, RECOVER, RECOVERY_SPEED, OPEN_SEAM, BORDER,
// MARGIN, FIXED, LAYER_RELAXATION, TURNED.

struct Step {
    // xyz: gravity (m/s²), w: substep h (s).
    gravity_h: vec4f,
    // x: seam compliance / h², y: grab compliance / h², z: 1 / h², w: the
    // substep's index in the frame.
    params: vec4f,
    // xyz: where the grab pulls its particle.
    grab_target: vec4f,
    // x: how far the sewing ramp has gone (0–1).
    sewing: vec4f,
}

struct Range {
    start: u32,
    count: u32,
    _pad0: u32,
    _pad1: u32,
}

struct Scene {
    // xyz: SDF origin (m), w: node spacing (m).
    sdf_origin: vec4f,
    // xyz: bricks along x, y, z; w: 1 if there is a body.
    sdf_dims: vec4u,
    // x: self-collision radius (m), y: hash cell size (m), z: 1 to
    // collide with the ground, w: candidate reach (m).
    contact: vec4f,
    // x: particles, y: hash table size (power of two), z: grabbed
    // particles.
    counts: vec4u,
    // x: where the nodes' vertices start in `sdf_values`, y: bones of a
    // moving body (0: the body stands still), z: where the skin starts.
    body: vec4u,
    // x: how far the corners of a triangle of another garment a particle
    // may touch can be (m); 0: a single garment. y: the cloth's triangles
    // (before the seams' bridges; exact as a float).
    layers: vec4f,
}

// A triangle for contacts between garments: corners, then its garment
// (low 16 bits) and free edges (bits 16..24: bit k the edge opposite
// corner k, BORDER a corner on a free edge); for a seam's bridge, the
// cloth triangle it continues (NONE for the cloth's own).
struct LayerTri {
    v: vec4u,
    bridged: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct Membrane {
    v: vec4u,
    // cu.xy, cv.xy
    coef: vec4f,
    // warp, weft, shear compliances (m/N); w: 1 along an elastic (buckles)
    alpha: vec4f,
}

struct Hinge {
    v: vec4u,
    alpha: vec4f,
}

@group(0) @binding(0) var<uniform> step: Step;
@group(0) @binding(1) var<uniform> range: Range;
@group(0) @binding(2) var<uniform> scene: Scene;

@group(1) @binding(0) var<storage, read_write> x: array<vec4f>;
@group(1) @binding(1) var<storage, read_write> prev: array<vec4f>;
@group(1) @binding(2) var<storage, read_write> vel: array<vec4f>;
// Per particle: inverse mass, thickness, friction, damping.
@group(1) @binding(3) var<storage, read> pdata: array<vec4f>;
@group(1) @binding(4) var<storage, read> membranes: array<Membrane>;
@group(1) @binding(5) var<storage, read> hinges: array<Hinge>;
@group(1) @binding(6) var<storage, read> seams: array<vec2u>;
@group(1) @binding(7) var<storage, read_write> near: array<u32>;
// Per particle: xyz a correction; w a count or a mask (see the kernels).
@group(1) @binding(8) var<storage, read_write> delta: array<vec4f>;
// Compressed rows: row i is csr[csr[i]] .. csr[csr[i + 1]].
@group(1) @binding(9) var<storage, read> edge_csr: array<u32>;
@group(1) @binding(10) var<storage, read> sewn_csr: array<u32>;
@group(1) @binding(11) var<storage, read> excluded_csr: array<u32>;
@group(1) @binding(12) var<storage, read> sdf_slots: array<u32>;
// The field's values; then (from scene.body.x) per value the vertex nearest
// to the closest surface point (u32::MAX: none); then (from scene.body.z)
// per vertex its skin: four bone indices in one word, four weights (u32
// words as f32 bit patterns).
@group(1) @binding(13) var<storage, read> sdf_values: array<f32>;
// xyz: offset from the target, w: particle index (exact as a float).
@group(1) @binding(14) var<storage, read> grab: array<vec4f>;
@group(1) @binding(15) var<storage, read_write> cell_count: array<atomic<u32>>;
@group(1) @binding(16) var<storage, read_write> cell_start: array<u32>;
@group(1) @binding(17) var<storage, read_write> sorted: array<u32>;
@group(1) @binding(18) var<storage, read_write> neighbours: array<u32>;
@group(1) @binding(19) var<storage, read_write> neighbour_count: array<u32>;
// Per particle: how many candidates it overlaps this substep.
@group(1) @binding(20) var<storage, read_write> contacts: array<u32>;
// Per bone, the box (rest pose) of the nodes whose vertex it leads: low,
// high. Then per substep and bone, three rigid motions (rotation
// quaternion, then translation): the bone's at the substep's end, its
// inverse, and the bone's at the substep's start.
@group(1) @binding(21) var<storage, read> body: array<vec4f>;
@group(1) @binding(22) var<storage, read> closures: array<vec2u>;
// Per particle, its garment (numbered from the body out).
@group(1) @binding(23) var<storage, read> garment: array<u32>;
// Compressed rows: per particle, the triangles it is a corner of.
@group(1) @binding(24) var<storage, read> incident_csr: array<u32>;
@group(1) @binding(25) var<storage, read> triangles: array<LayerTri>;
// Per particle, a row of 1 + MAX_TOUCHES: how many triangles of other
// garments it may touch, then those triangles; after the rows, two flags
// (rebuild the hash, gather the candidates again), then per cloth
// triangle 1 if it is turned over (see `layer_turned`).
@group(1) @binding(26) var<storage, read_write> touches: array<u32>;
// Per particle, the sum of its pushes from contacts between garments
// (xyz, in units of FIXED m) and their count.
@group(1) @binding(27) var<storage, read_write> layer_delta: array<atomic<i32>>;
// Where each particle was when the candidates were gathered.
@group(1) @binding(28) var<storage, read_write> gathered: array<vec4f>;
// Elastic segments (r3d-sim `ElasticLink`).
struct Elastic {
    v: vec2u,
    // Length on the pattern, relaxed length (m), 1 / stiffness (1/N).
    full: f32,
    rest: f32,
    inv_stiffness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(1) @binding(29) var<storage, read> elastics: array<Elastic>;
// Per particle: the elastics' pull on it this substep (N), read and
// cleared by `collide` (r3d-sim `CpuSolver::press`).
@group(1) @binding(30) var<storage, read_write> press: array<vec4f>;

const WG: u32 = 64u;

fn finite(v: f32) -> bool {
    return v == v && abs(v) < 1e30;
}

// ---------------------------------------------------------------------------
// Prediction and velocities

@compute @workgroup_size(WG)
fn predict(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let p = x[i];
    prev[i] = p;
    if pdata[i].x > 0.0 {
        let h = step.gravity_h.w;
        let v = vel[i].xyz + step.gravity_h.xyz * h;
        vel[i] = vec4f(v, 0.0);
        x[i] = vec4f(p.xyz + v * h, 0.0);
    }
}

@compute @workgroup_size(WG)
fn velocity(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let h = step.gravity_h.w;
    let v = (x[i].xyz - prev[i].xyz) / h;
    vel[i] = vec4f(v * max(1.0 - pdata[i].w * h, 0.0), 0.0);
}

// ---------------------------------------------------------------------------
// Membranes (Strain Based Dynamics, Müller et al. 2014)

// One XPBD update on three particles; returns the multiplier step (0 if
// the constraint is degenerate).
fn dlambda3(c: f32, g: array<vec3f, 3>, w: vec3f, alpha_tilde: f32) -> f32 {
    let denom = w.x * dot(g[0], g[0]) + w.y * dot(g[1], g[1]) + w.z * dot(g[2], g[2]) + alpha_tilde;
    if denom <= 1e-12 || !finite(c) {
        return 0.0;
    }
    return -c / denom;
}

@compute @workgroup_size(WG)
fn membrane(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= range.count {
        return;
    }
    let m = membranes[range.start + id.x];
    let w = vec3f(pdata[m.v.x].x, pdata[m.v.y].x, pdata[m.v.z].x);
    var p0 = x[m.v.x].xyz;
    var p1 = x[m.v.y].xyz;
    var p2 = x[m.v.z].xyz;
    let cu = m.coef.xy;
    let cv = m.coef.zw;
    let inv_h2 = step.params.z;
    // Stretch along the warp, then the weft.
    for (var k = 0u; k < 2u; k++) {
        let coef = select(cu, cv, k == 1u);
        let alpha = select(m.alpha.x, m.alpha.y, k == 1u) * inv_h2;
        let f = (p1 - p0) * coef.x + (p2 - p0) * coef.y;
        let len = length(f);
        // Along an elastic the cloth resists stretching only (it gathers).
        let buckled = m.alpha.w > 0.5 && len < 1.0;
        if len > 1e-9 && !buckled {
            let n = f / len;
            let g1 = n * coef.x;
            let g2 = n * coef.y;
            let g = array<vec3f, 3>(-(g1 + g2), g1, g2);
            let dl = dlambda3(len - 1.0, g, w, alpha);
            p0 += g[0] * (dl * w.x);
            p1 += g[1] * (dl * w.y);
            p2 += g[2] * (dl * w.z);
        }
    }
    // Shear.
    let fu = (p1 - p0) * cu.x + (p2 - p0) * cu.y;
    let fv = (p1 - p0) * cv.x + (p2 - p0) * cv.y;
    let g1 = fv * cu.x + fu * cv.x;
    let g2 = fv * cu.y + fu * cv.y;
    let g = array<vec3f, 3>(-(g1 + g2), g1, g2);
    let dl = dlambda3(dot(fu, fv), g, w, m.alpha.z * inv_h2);
    p0 += g[0] * (dl * w.x);
    p1 += g[1] * (dl * w.y);
    p2 += g[2] * (dl * w.z);
    x[m.v.x] = vec4f(p0, 0.0);
    x[m.v.y] = vec4f(p1, 0.0);
    x[m.v.z] = vec4f(p2, 0.0);
}

// ---------------------------------------------------------------------------
// Bending: dihedral angle, gradients from Bridson et al. 2003

@compute @workgroup_size(WG)
fn hinge(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= range.count {
        return;
    }
    let hg = hinges[range.start + id.x];
    let p0 = x[hg.v.x].xyz;
    let p1 = x[hg.v.y].xyz;
    let p2 = x[hg.v.z].xyz;
    let p3 = x[hg.v.w].xyz;
    let n1 = cross(p2 - p0, p3 - p0);
    let n2 = cross(p3 - p1, p2 - p1);
    let e = p3 - p2;
    let l1 = dot(n1, n1);
    let l2 = dot(n2, n2);
    let le = length(e);
    if l1 < 1e-20 || l2 < 1e-20 || le < 1e-10 {
        return;
    }
    let m1 = n1 / sqrt(l1);
    let m2 = n2 / sqrt(l2);
    let theta = atan2(dot(cross(m2, m1), e / le), dot(m1, m2));
    let q1 = n1 / l1;
    let q2 = n2 / l2;
    let u0 = q1 * le;
    let u1 = q2 * le;
    let u2 = q1 * (dot(p0 - p3, e) / le) + q2 * (dot(p1 - p3, e) / le);
    let u3 = -q1 * (dot(p0 - p2, e) / le) - q2 * (dot(p1 - p2, e) / le);
    let w = vec4f(pdata[hg.v.x].x, pdata[hg.v.y].x, pdata[hg.v.z].x, pdata[hg.v.w].x);
    let denom = w.x * dot(u0, u0) + w.y * dot(u1, u1) + w.z * dot(u2, u2) + w.w * dot(u3, u3)
        + hg.alpha.x * step.params.z;
    if denom <= 1e-12 || !finite(theta) {
        return;
    }
    let dl = -theta / denom;
    x[hg.v.x] = vec4f(p0 + u0 * (dl * w.x), 0.0);
    x[hg.v.y] = vec4f(p1 + u1 * (dl * w.y), 0.0);
    x[hg.v.z] = vec4f(p2 + u2 * (dl * w.z), 0.0);
    x[hg.v.w] = vec4f(p3 + u3 * (dl * w.w), 0.0);
}

// ---------------------------------------------------------------------------
// Seams: zero-length distances

@compute @workgroup_size(WG)
fn seam(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= range.count {
        return;
    }
    let s = seams[range.start + id.x];
    let pa = x[s.x].xyz;
    let pb = x[s.y].xyz;
    let d = pb - pa;
    let len = length(d);
    let wa = pdata[s.x].x;
    let wb = pdata[s.y].x;
    let denom = wa + wb + step.params.x;
    if len <= 1e-9 || denom <= 1e-12 {
        return;
    }
    let n = d / len;
    let dl = -len / denom;
    x[s.x] = vec4f(pa - n * (dl * wa), 0.0);
    x[s.y] = vec4f(pb + n * (dl * wb), 0.0);
}

// ---------------------------------------------------------------------------
// Closures (buttons, zippers): stiff distances whose target shrinks to
// zero at CLOSING_SPEED from the gap at the substep's start (r3d-sim
// `solver::closure_target`)

@compute @workgroup_size(WG)
fn closure(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= range.count {
        return;
    }
    let s = closures[range.start + id.x];
    let pa = x[s.x].xyz;
    let pb = x[s.y].xyz;
    let d = pb - pa;
    let len = length(d);
    let wa = pdata[s.x].x;
    let wb = pdata[s.y].x;
    let denom = wa + wb + CLOSURE_COMPLIANCE * step.params.z;
    if len <= 1e-9 || denom <= 1e-12 {
        return;
    }
    let goal = max(distance(prev[s.x].xyz, prev[s.y].xyz) - CLOSING_SPEED * step.gravity_h.w, 0.0);
    let n = d / len;
    let dl = (len - goal) / denom;
    x[s.x] = vec4f(pa + n * (dl * wa), 0.0);
    x[s.y] = vec4f(pb - n * (dl * wb), 0.0);
}

// ---------------------------------------------------------------------------
// Elastics: distances that only pull, their target going from the length
// on the pattern to the relaxed length over the sewing ramp (r3d-sim
// `solver::elastic_target`), with the compliance goal / stiffness

@compute @workgroup_size(WG)
fn elastic(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= range.count {
        return;
    }
    let e = elastics[range.start + id.x];
    let pa = x[e.v.x].xyz;
    let pb = x[e.v.y].xyz;
    let d = pb - pa;
    let len = length(d);
    let goal = e.full + (e.rest - e.full) * step.sewing.x;
    if len <= goal || len <= 1e-9 {
        return;
    }
    let wa = pdata[e.v.x].x;
    let wb = pdata[e.v.y].x;
    let denom = wa + wb + goal * e.inv_stiffness * step.params.z;
    if denom <= 1e-12 {
        return;
    }
    let n = d / len;
    let dl = (len - goal) / denom;
    x[e.v.x] = vec4f(pa + n * (dl * wa), 0.0);
    x[e.v.y] = vec4f(pb - n * (dl * wb), 0.0);
    // Coloured: no other segment of this dispatch touches either end.
    let tension = n * ((len - goal) / (goal * e.inv_stiffness));
    press[e.v.x] = vec4f(press[e.v.x].xyz + tension, 0.0);
    press[e.v.y] = vec4f(press[e.v.y].xyz - tension, 0.0);
}

// ---------------------------------------------------------------------------
// Grab: a soft zero-length pull of each held particle to the target

@compute @workgroup_size(WG)
fn pull(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= scene.counts.z {
        return;
    }
    let g = grab[id.x];
    let i = u32(g.w);
    let w = pdata[i].x;
    let p = x[i].xyz;
    x[i] = vec4f(p + (step.grab_target.xyz + g.xyz - p) * (w / (w + step.params.y)), 0.0);
}

// ---------------------------------------------------------------------------
// The body's signed distance field (r3d-sim `collide::StaticMesh`)

fn sdf_node(c: vec3u) -> f32 {
    let b = c / BRICK;
    let dims = scene.sdf_dims.xyz;
    if any(b >= dims) {
        return BAND;
    }
    let slot = sdf_slots[(b.z * dims.y + b.y) * dims.x + b.x];
    if slot == 0xffffffffu {
        return BAND;
    }
    let l = c % BRICK;
    return sdf_values[slot * BRICK * BRICK * BRICK + (l.z * BRICK + l.y) * BRICK + l.x];
}

// xyz: outward normal, w: signed distance; w = BAND when out of the band.
fn sdf_sample(p: vec3f) -> vec4f {
    let none = vec4f(0.0, 0.0, 0.0, BAND);
    if scene.sdf_dims.w == 0u {
        return none;
    }
    let q = (p - scene.sdf_origin.xyz) / scene.sdf_origin.w;
    if min(q.x, min(q.y, q.z)) < 0.0 || max(q.x, max(q.y, q.z)) > 1e7 {
        return none;
    }
    let c = vec3u(q);
    let f = q - vec3f(c);
    let c000 = sdf_node(c);
    let c100 = sdf_node(c + vec3u(1u, 0u, 0u));
    let c010 = sdf_node(c + vec3u(0u, 1u, 0u));
    let c110 = sdf_node(c + vec3u(1u, 1u, 0u));
    let c001 = sdf_node(c + vec3u(0u, 0u, 1u));
    let c101 = sdf_node(c + vec3u(1u, 0u, 1u));
    let c011 = sdf_node(c + vec3u(0u, 1u, 1u));
    let c111 = sdf_node(c + vec3u(1u, 1u, 1u));
    if min(min(min(c000, c100), min(c010, c110)), min(min(c001, c101), min(c011, c111))) >= BAND {
        return none;
    }
    let x00 = mix(c000, c100, f.x);
    let x10 = mix(c010, c110, f.x);
    let x01 = mix(c001, c101, f.x);
    let x11 = mix(c011, c111, f.x);
    let y0 = mix(x00, x10, f.y);
    let y1 = mix(x01, x11, f.y);
    let distance = mix(y0, y1, f.z);
    let dx = mix(mix(c100 - c000, c110 - c010, f.y), mix(c101 - c001, c111 - c011, f.y), f.z);
    let dy = mix(x10 - x00, x11 - x01, f.z);
    let dz = y1 - y0;
    let g = vec3f(dx, dy, dz);
    let l = length(g);
    if !(l > 1e-12) {
        return none;
    }
    return vec4f(g / l, distance);
}

// The skin weights of the vertex recorded at the node nearest `p`
// (r3d-sim `StaticMesh::skin_at`): four bone indices, eight bits each,
// and their weights.
struct SkinAt {
    found: bool,
    bones: u32,
    w: vec4f,
}

fn skin_at(p: vec3f) -> SkinAt {
    let none = SkinAt(false, 0u, vec4f(0.0));
    let q = round((p - scene.sdf_origin.xyz) / scene.sdf_origin.w);
    if min(q.x, min(q.y, q.z)) < 0.0 || max(q.x, max(q.y, q.z)) > 1e7 {
        return none;
    }
    let c = vec3u(q);
    let b = c / BRICK;
    let dims = scene.sdf_dims.xyz;
    if any(b >= dims) {
        return none;
    }
    let slot = sdf_slots[(b.z * dims.y + b.y) * dims.x + b.x];
    if slot == 0xffffffffu {
        return none;
    }
    let l = c % BRICK;
    let k = slot * BRICK * BRICK * BRICK + (l.z * BRICK + l.y) * BRICK + l.x;
    let v = bitcast<u32>(sdf_values[scene.body.x + k]);
    if v == 0xffffffffu {
        return none;
    }
    let s = scene.body.z + 5u * v;
    let w = vec4f(sdf_values[s + 1u], sdf_values[s + 2u], sdf_values[s + 3u], sdf_values[s + 4u]);
    return SkinAt(true, bitcast<u32>(sdf_values[s]), w);
}

// The bones that lead a vertex (r3d-sim `Skin::leading`), one bit each.
fn leading(s: SkinAt) -> u32 {
    let heaviest = max(max(s.w.x, s.w.y), max(s.w.z, s.w.w));
    var mask = 0u;
    for (var i = 0u; i < 4u; i++) {
        let bone = (s.bones >> (8u * i)) & 0xffu;
        if s.w[i] > 0.0 && s.w[i] >= 0.5 * heaviest && bone < 32u {
            mask |= 1u << bone;
        }
    }
    return mask;
}

fn rotate(q: vec4f, v: vec3f) -> vec3f {
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

// Rigid motion `k` of the body buffer applied to `p`.
fn rigid(k: u32, p: vec3f) -> vec3f {
    return rotate(body[k], p) + body[k + 1u].xyz;
}

// A vertex's skinning `p ↦ M p + t`, M by its columns; `ok` is false when
// a bone is missing or M is singular.
struct Blend {
    c0: vec3f,
    c1: vec3f,
    c2: vec3f,
    t: vec3f,
    ok: bool,
}

// `which`: 0 for the bones' motions at the substep's end, 4 at its start.
fn blend(s: SkinAt, base: u32, which: u32) -> Blend {
    var m = Blend(vec3f(0.0), vec3f(0.0), vec3f(0.0), vec3f(0.0), true);
    for (var i = 0u; i < 4u; i++) {
        let w = s.w[i];
        if w > 0.0 {
            let bone = (s.bones >> (8u * i)) & 0xffu;
            if bone >= scene.body.y {
                m.ok = false;
                return m;
            }
            let k = base + bone * 6u + which;
            m.c0 += rotate(body[k], vec3f(1.0, 0.0, 0.0)) * w;
            m.c1 += rotate(body[k], vec3f(0.0, 1.0, 0.0)) * w;
            m.c2 += rotate(body[k], vec3f(0.0, 0.0, 1.0)) * w;
            m.t += body[k + 1u].xyz * w;
        }
    }
    m.ok = abs(dot(m.c0, cross(m.c1, m.c2))) > 1e-3;
    return m;
}

fn apply_blend(m: Blend, q: vec3f) -> vec3f {
    return m.c0 * q.x + m.c1 * q.y + m.c2 * q.z + m.t;
}

// M⁻¹ y.
fn solve(m: Blend, y: vec3f) -> vec3f {
    let d = dot(m.c0, cross(m.c1, m.c2));
    return vec3f(dot(cross(m.c1, m.c2), y), dot(cross(m.c2, m.c0), y), dot(cross(m.c0, m.c1), y)) / d;
}

// M⁻ᵀ n.
fn inverse_transpose(m: Blend, n: vec3f) -> vec3f {
    let d = dot(m.c0, cross(m.c1, m.c2));
    return (cross(m.c1, m.c2) * n.x + cross(m.c2, m.c0) * n.y + cross(m.c0, m.c1) * n.z) / d;
}

// The body seen from `p` (r3d-sim `motion::touch`).
struct Touch {
    // xyz: outward normal, w: signed distance; w = BAND when out of the band.
    c: vec4f,
    // How far the body's surface there moved this substep.
    moved: vec3f,
}

fn touch(p: vec3f) -> Touch {
    let parts = scene.body.y;
    if parts == 0u {
        return Touch(sdf_sample(p), vec3f(0.0));
    }
    var best = Touch(vec4f(0.0, 0.0, 0.0, BAND), vec3f(0.0));
    let base = 2u * parts + u32(step.params.w) * parts * 6u;
    for (var part = 0u; part < parts; part++) {
        var q = rigid(base + part * 6u + 2u, p);
        if any(q < body[2u * part].xyz) || any(q > body[2u * part + 1u].xyz) {
            continue;
        }
        let start = skin_at(q);
        if !start.found || (leading(start) & (1u << part)) == 0u {
            continue;
        }
        var solved = true;
        for (var i = 0u; i < ITERATIONS; i++) {
            let s = skin_at(q);
            let m = blend(s, base, 0u);
            if !s.found || !m.ok {
                solved = false;
                break;
            }
            q = solve(m, p - m.t);
        }
        let s = skin_at(q);
        if !solved || !s.found {
            continue;
        }
        let m = blend(s, base, 0u);
        let mb = blend(s, base, 4u);
        if !m.ok || !mb.ok || distance(apply_blend(m, q), p) > ON_SKIN {
            continue;
        }
        let c = sdf_sample(q);
        if c.w >= BAND {
            continue;
        }
        let n = inverse_transpose(m, c.xyz);
        let stretch = length(n);
        if !(stretch > 1e-6) {
            continue;
        }
        let d = c.w / stretch;
        if d >= best.c.w {
            continue;
        }
        best = Touch(vec4f(n / stretch, d), p - apply_blend(mb, q));
    }
    return best;
}

// ---------------------------------------------------------------------------
// Contacts with the body and the ground, with Coulomb friction in
// positions (Macklin et al. 2014, §6.1)

@compute @workgroup_size(WG)
fn collide(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    near[i] = 0u;
    let pressing = press[i].xyz;
    press[i] = vec4f(0.0);
    let d = pdata[i];
    if d.x == 0.0 {
        return;
    }
    var p = x[i].xyz;
    var normal = vec3f(0.0);
    var depth = 0.0;
    var touching = false;
    // How far the surface touched moved this substep: friction acts on
    // the sliding relative to it.
    var carried = vec3f(0.0);
    let distance = d.y + SKIN;
    let t = touch(p);
    let c = t.c;
    if c.w < BAND && c.w < distance + NEAR {
        near[i] = 1u;
        if c.w < distance {
            depth = distance - c.w;
            p += c.xyz * depth;
            normal = c.xyz;
            carried = t.moved;
            touching = true;
            // Pre-stabilisation, as on the CPU: no more speed along the
            // normal than the surface touched has (a moving body only).
            let gained = dot(p - prev[i].xyz, normal);
            let allowed = max(dot(carried, normal), 0.0);
            if scene.body.y > 0u && gained > allowed {
                prev[i] = vec4f(prev[i].xyz + normal * (gained - allowed), 0.0);
            }
        }
    }
    if scene.contact.z > 0.5 && p.y < d.y {
        depth = d.y - p.y;
        p.y = d.y;
        normal = vec3f(0.0, 1.0, 0.0);
        carried = vec3f(0.0);
        touching = true;
    }
    if touching {
        // An elastic's pull into the surface counts as the displacement it
        // would give in a substep (as on the CPU).
        let h = step.gravity_h.w;
        let pressed = max(-dot(pressing, normal), 0.0) * d.x * h * h;
        let moved = p - prev[i].xyz - carried;
        let tangential = moved - normal * dot(moved, normal);
        let sliding = length(tangential);
        if sliding > 0.0 {
            p -= tangential * min(d.z * (depth + pressed) / sliding, 1.0);
        }
    }
    x[i] = vec4f(p, 0.0);
}

// Edges between two particles near the body keep their middle out of it,
// pushing both ends by their inverse masses; a particle's pushes from its
// edges are averaged (constraint averaging, Macklin et al. 2014 §4.2).
@compute @workgroup_size(WG)
fn edges(@builtin(global_invocation_id) id: vec3u) {
    let a = id.x;
    if a >= scene.counts.x {
        return;
    }
    var sum = vec3f(0.0);
    var count = 0u;
    let wa = pdata[a].x;
    if near[a] != 0u && wa > 0.0 {
        let xa = x[a].xyz;
        for (var k = edge_csr[a]; k < edge_csr[a + 1u]; k++) {
            let b = edge_csr[k];
            if near[b] == 0u {
                continue;
            }
            let wb = pdata[b].x;
            let middle = (xa + x[b].xyz) * 0.5;
            let distance = (pdata[a].y + pdata[b].y) * 0.5 + SKIN;
            let c = touch(middle).c;
            if c.w < BAND && c.w < distance {
                sum += c.xyz * (2.0 * (distance - c.w) / (wa + wb) * wa);
                count += 1u;
            }
        }
    }
    delta[a] = vec4f(sum / f32(max(count, 1u)), 0.0);
}

@compute @workgroup_size(WG)
fn apply(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    x[i] = vec4f(x[i].xyz + delta[i].xyz, 0.0);
}

// ---------------------------------------------------------------------------
// Self-collision (Müller et al. 2014): particle spheres, candidates from a
// spatial hash (Teschner et al. 2003) gathered once per frame.

fn cell_of(p: vec3f) -> vec3i {
    return vec3i(floor(p / scene.contact.y));
}

fn hash(c: vec3i) -> u32 {
    let u = bitcast<vec3u>(c);
    return ((u.x * 73856093u) ^ (u.y * 19349663u) ^ (u.z * 83492791u)) & (scene.counts.y - 1u);
}

// The hash is rebuilt only when asked (the first flag after the rows of
// `touches`): at the start of a frame, and when the cloth has moved
// since the candidates between garments were gathered.
fn rehash() -> bool {
    return touches[scene.counts.x * (MAX_TOUCHES + 1u)] != 0u;
}

@compute @workgroup_size(WG)
fn hash_clear(@builtin(global_invocation_id) id: vec3u) {
    if id.x < scene.counts.y && rehash() {
        atomicStore(&cell_count[id.x], 0u);
    }
}

@compute @workgroup_size(WG)
fn hash_count(@builtin(global_invocation_id) id: vec3u) {
    if id.x < scene.counts.x && rehash() {
        atomicAdd(&cell_count[hash(cell_of(x[id.x].xyz))], 1u);
    }
}

const SCAN: u32 = 256u;
var<workgroup> partial: array<u32, SCAN>;
var<workgroup> scan_wanted: u32;

// Exclusive prefix sum of the cell counts into `cell_start` (one
// workgroup; each thread scans a chunk); the counts become write cursors.
@compute @workgroup_size(SCAN)
fn hash_scan(@builtin(local_invocation_id) lid: vec3u) {
    let t = lid.x;
    if t == 0u {
        scan_wanted = select(0u, 1u, rehash());
    }
    if workgroupUniformLoad(&scan_wanted) == 0u {
        return;
    }
    let size = scene.counts.y;
    let chunk = size / SCAN;
    var sum = 0u;
    for (var k = 0u; k < chunk; k++) {
        sum += atomicLoad(&cell_count[t * chunk + k]);
    }
    partial[t] = sum;
    workgroupBarrier();
    // Hillis–Steele inclusive scan of the chunk sums.
    for (var offset = 1u; offset < SCAN; offset *= 2u) {
        var v = partial[t];
        if t >= offset {
            v += partial[t - offset];
        }
        workgroupBarrier();
        partial[t] = v;
        workgroupBarrier();
    }
    var running = partial[t] - sum;
    for (var k = 0u; k < chunk; k++) {
        let c = t * chunk + k;
        let n = atomicLoad(&cell_count[c]);
        cell_start[c] = running;
        atomicStore(&cell_count[c], running);
        running += n;
    }
    if t == SCAN - 1u {
        cell_start[size] = running;
    }
}

@compute @workgroup_size(WG)
fn hash_scatter(@builtin(global_invocation_id) id: vec3u) {
    if id.x < scene.counts.x && rehash() {
        let slot = atomicAdd(&cell_count[hash(cell_of(x[id.x].xyz))], 1u);
        sorted[slot] = id.x;
    }
}

fn is_excluded(i: u32, j: u32) -> bool {
    var lo = excluded_csr[i];
    var hi = excluded_csr[i + 1u];
    while lo < hi {
        let mid = (lo + hi) / 2u;
        let v = excluded_csr[mid];
        if v == j {
            return true;
        }
        if v < j {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    return false;
}

// The candidates of each particle, sorted by index (deterministic, and
// a cell seen twice through a hash collision adds nothing); past
// MAX_NEIGHBOURS the highest indices are dropped.
@compute @workgroup_size(WG)
fn find_neighbours(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let p = x[i].xyz;
    let wi = pdata[i].x;
    let reach2 = scene.contact.w * scene.contact.w;
    let c = cell_of(p);
    var list: array<u32, MAX_NEIGHBOURS>;
    var n = 0u;
    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let h = hash(c + vec3i(dx, dy, dz));
                for (var k = cell_start[h]; k < cell_start[h + 1u]; k++) {
                    let j = sorted[k];
                    if j == i || wi + pdata[j].x == 0.0 || garment[j] != garment[i] {
                        continue;
                    }
                    let d = x[j].xyz - p;
                    if dot(d, d) > reach2 || is_excluded(i, j) {
                        continue;
                    }
                    // Insert j in order.
                    var at = n;
                    for (var s = 0u; s < n; s++) {
                        if list[s] >= j {
                            at = s;
                            break;
                        }
                    }
                    if (at < n && list[at] == j) || at >= MAX_NEIGHBOURS {
                        continue;
                    }
                    var s = min(n, MAX_NEIGHBOURS - 1u);
                    while s > at {
                        list[s] = list[s - 1u];
                        s--;
                    }
                    list[at] = j;
                    n = min(n + 1u, MAX_NEIGHBOURS);
                }
            }
        }
    }
    for (var s = 0u; s < n; s++) {
        neighbours[i * MAX_NEIGHBOURS + s] = list[s];
    }
    neighbour_count[i] = n;
}

// Inverse mass of i with the particles sewn to it that are within a radius
// (the seam has closed, they move as one); 0 if any is pinned.
fn welded_w(i: u32, r2: f32) -> f32 {
    let wi = pdata[i].x;
    if wi == 0.0 {
        return 0.0;
    }
    var mass = 1.0 / wi;
    let p = x[i].xyz;
    for (var k = sewn_csr[i]; k < sewn_csr[i + 1u]; k++) {
        let j = sewn_csr[k];
        let d = x[j].xyz - p;
        if dot(d, d) <= r2 {
            let wj = pdata[j].x;
            if wj == 0.0 {
                return 0.0;
            }
            mass += 1.0 / wj;
        }
    }
    return 1.0 / mass;
}

// Whether i and j overlap and can be pushed apart.
fn overlaps(i: u32, j: u32) -> bool {
    let d = x[i].xyz - x[j].xyz;
    let len = length(d);
    return len < 2.0 * scene.contact.x && len >= 1e-9 && pdata[i].x + pdata[j].x > 0.0;
}

@compute @workgroup_size(WG)
fn self_count(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    var count = 0u;
    for (var s = 0u; s < neighbour_count[i]; s++) {
        if overlaps(i, neighbours[i * MAX_NEIGHBOURS + s]) {
            count += 1u;
        }
    }
    contacts[i] = count;
}

// Each particle's share of the pushes out of its overlapping candidates.
// A pair is scaled by 1 / the larger contact count of its two particles,
// the same on both sides: averaging each particle by its own count moved
// the two sides of a pair by unequal impulses, and tangled cloth (a sheet
// hanging from a held corner) propelled itself.
// w: the mask of its welded seam partners (bit k: row entry k; up to 24,
// exact as a float).
@compute @workgroup_size(WG)
fn self_gather(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let r = scene.contact.x;
    let r2 = r * r;
    let min_d = 2.0 * r;
    let p = x[i].xyz;
    var mask = 0u;
    let first = sewn_csr[i];
    for (var k = first; k < sewn_csr[i + 1u] && k - first < 24u; k++) {
        let d = x[sewn_csr[k]].xyz - p;
        if dot(d, d) <= r2 {
            mask |= 1u << (k - first);
        }
    }
    var sum = vec3f(0.0);
    let ci = contacts[i];
    if ci > 0u {
        let wi = welded_w(i, r2);
        for (var s = 0u; s < neighbour_count[i]; s++) {
            let j = neighbours[i * MAX_NEIGHBOURS + s];
            if !overlaps(i, j) {
                continue;
            }
            let wj = welded_w(j, r2);
            if wi + wj == 0.0 {
                continue;
            }
            let d = p - x[j].xyz;
            let len = length(d);
            sum += d / len * ((min_d - len) * wi / (wi + wj) / f32(max(ci, contacts[j])));
        }
    }
    delta[i] = vec4f(sum, f32(mask));
}

// A particle moves by its own push and those of its welded partners.
@compute @workgroup_size(WG)
fn self_apply(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    var total = delta[i].xyz;
    let mask = u32(delta[i].w);
    let first = sewn_csr[i];
    for (var k = first; k < sewn_csr[i + 1u] && k - first < 24u; k++) {
        if (mask & (1u << (k - first))) != 0u {
            total += delta[sewn_csr[k]].xyz;
        }
    }
    x[i] = vec4f(x[i].xyz + total, 0.0);
}

// ---------------------------------------------------------------------------
// Contacts between garments (r3d-sim's `layers.rs`; Bridson et al. 2002):
// a particle stays on its side of the triangles of other garments (the
// outer garment on the face an inner one's triangles show), a few
// millimetres away. Candidates are gathered from the spatial hash when
// the cloth has moved MARGIN / 2 since they last were (checked at the
// start of a frame and every few substeps; the flags after the rows of
// `touches`). The pushes are scattered to the triangles' corners as
// integers (sums independent of their order) and averaged per particle.

// +1 if particle i (of garment g) belongs on the side triangle t faces.
fn layer_side(g: u32, t: LayerTri) -> f32 {
    return select(-1.0, 1.0, g > (t.v.w & 0xffffu));
}

fn layer_gap(i: u32, t: LayerTri) -> f32 {
    let sheet = (pdata[t.v.x].y + pdata[t.v.y].y + pdata[t.v.z].y) / 3.0;
    return pdata[i].y + sheet + LAYER_SKIN;
}

fn free_bits(t: LayerTri) -> u32 {
    return (t.v.w >> 16u) & 0xffu;
}

// The garment of particle i (that of its first triangle); NONE if it is
// in no triangle.
const NONE: u32 = 0xffffffffu;
fn garment_of(i: u32) -> u32 {
    if incident_csr[i] == incident_csr[i + 1u] {
        return NONE;
    }
    return triangles[incident_csr[incident_csr[i]]].v.w & 0xffffu;
}

struct Over {
    ok: bool,
    n: vec3f,
    bary: vec3f,
    d: f32,
}

// Where p lies over triangle (a, b, c): its unit normal, the barycentric
// weights of p's foot (clamped to the triangle) and p's signed distance;
// not ok for a degenerate triangle, or p farther past an edge than
// OVERHANG, or past a free one (bit k of `free`: the edge opposite
// corner k).
fn over(p: vec3f, a: vec3f, b: vec3f, c: vec3f, free: u32) -> Over {
    var out: Over;
    out.ok = false;
    let e1 = b - a;
    let e2 = c - a;
    let nn = cross(e1, e2);
    let area2 = dot(nn, nn);
    if area2 < 1e-16 {
        return out;
    }
    let rel = p - a;
    let d00 = dot(e1, e1);
    let d01 = dot(e1, e2);
    let d11 = dot(e2, e2);
    let d20 = dot(rel, e1);
    let d21 = dot(rel, e2);
    let det = d00 * d11 - d01 * d01;
    if abs(det) < 1e-20 {
        return out;
    }
    let v = (d11 * d20 - d01 * d21) / det;
    let w = (d00 * d21 - d01 * d20) / det;
    let u = 1.0 - v - w;
    let lim = vec3f(
        select(-OVERHANG, 0.0, (free & 1u) != 0u),
        select(-OVERHANG, 0.0, (free & 2u) != 0u),
        select(-OVERHANG, 0.0, (free & 4u) != 0u),
    );
    if u < lim.x || v < lim.y || w < lim.z {
        return out;
    }
    let b3 = max(vec3f(u, v, w), vec3f(0.0));
    let n = nn / sqrt(area2);
    out.ok = true;
    out.n = n;
    out.bary = b3 / (b3.x + b3.y + b3.z);
    out.d = dot(rel, n);
    return out;
}

// Whether triangle t holds cloth off now: always for the cloth's own; a
// seam's bridge while the seam is open wider than OPEN_SEAM and it faces
// as the cloth triangle it continues.
fn holds(t: LayerTri) -> bool {
    if t.bridged == NONE {
        return true;
    }
    let a = x[t.v.x].xyz;
    let b = x[t.v.y].xyz;
    let c = x[t.v.z].xyz;
    let r = triangles[t.bridged].v;
    let ra = x[r.x].xyz;
    let facing = cross(x[r.y].xyz - ra, x[r.z].xyz - ra);
    let fl = length(facing);
    if fl < 1e-12 {
        return false;
    }
    let longest = max(max(length(b - a), length(c - b)), length(a - c));
    return dot(cross(b - a, c - a), facing / fl) > OPEN_SEAM * longest;
}

fn flag_at() -> u32 {
    return scene.counts.x * (MAX_TOUCHES + 1u);
}

// Ask for the hash (the self-collision candidates need it every frame).
@compute @workgroup_size(1)
fn layer_mark() {
    touches[flag_at()] = 1u;
}

@compute @workgroup_size(1)
fn layer_reset() {
    touches[flag_at()] = 0u;
    touches[flag_at() + 1u] = 0u;
}

// Whether each cloth triangle is turned over: near the body, its normal
// points into it (cosine below TURNED). Which side of it another garment
// belongs on is then not known: it takes no part (r3d-sim's `layers.rs`).
@compute @workgroup_size(WG)
fn layer_turned(@builtin(global_invocation_id) id: vec3u) {
    let k = id.x;
    if f32(k) >= scene.layers.y || touches[flag_at() + 1u] == 0u {
        return;
    }
    let t = triangles[k].v;
    let a = x[t.x].xyz;
    let b = x[t.y].xyz;
    let c = x[t.z].xyz;
    let nn = cross(b - a, c - a);
    let l = length(nn);
    let g = touch((a + b + c) / 3.0).c;
    touches[flag_at() + 2u + k] = select(0u, 1u, l > 1e-12 && g.w < BAND && dot(nn / l, g.xyz) < TURNED);
}

// Raise both flags if a particle moved MARGIN / 2 since the candidates
// were gathered (every particle that does writes the same 1s). They start
// raised, so the first frame gathers.
@compute @workgroup_size(WG)
fn layer_moved(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let d = x[i].xyz - gathered[i].xyz;
    if dot(d, d) > 0.25 * MARGIN * MARGIN {
        touches[flag_at()] = 1u;
        touches[flag_at() + 1u] = 1u;
    }
}

// Each particle's candidate triangles of other garments, sorted: those
// it may touch on its side, and those it lies on the wrong side of (from
// up to RECOVER deep, not at a free edge) unless some of that garment's
// triangles have it on their right side (then it is over a fold of that
// garment turned over).
@compute @workgroup_size(WG)
fn layer_find(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x || touches[flag_at() + 1u] == 0u {
        return;
    }
    let row = i * (MAX_TOUCHES + 1u);
    let p = x[i].xyz;
    gathered[i] = vec4f(p, 0.0);
    let own = garment_of(i);
    if own == NONE {
        touches[row] = 0u;
        return;
    }
    let reach = scene.layers.x;
    let reach2 = reach * reach;
    let wi = pdata[i].x;
    // Triangle index; bit 31: the particle is on its right side.
    var list: array<u32, MAX_TOUCHES>;
    var n = 0u;
    let c = cell_of(p);
    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let h = hash(c + vec3i(dx, dy, dz));
                for (var k = cell_start[h]; k < cell_start[h + 1u]; k++) {
                    let j = sorted[k];
                    let dj = x[j].xyz - p;
                    if dot(dj, dj) > reach2 {
                        continue;
                    }
                    for (var r = incident_csr[j]; r < incident_csr[j + 1u]; r++) {
                        let tk = incident_csr[r];
                        let t = triangles[tk];
                        if (t.v.w & 0xffffu) == own {
                            break;
                        }
                        var at = n;
                        var seen = false;
                        for (var s = 0u; s < n; s++) {
                            let e = list[s] & 0x7fffffffu;
                            if e == tk {
                                seen = true;
                                break;
                            }
                            if e > tk {
                                at = s;
                                break;
                            }
                        }
                        if seen || at >= MAX_TOUCHES {
                            continue;
                        }
                        let turned = f32(tk) < scene.layers.y && touches[flag_at() + 2u + tk] != 0u;
                        if wi + pdata[t.v.x].x + pdata[t.v.y].x + pdata[t.v.z].x == 0.0 || turned || !holds(t) {
                            continue;
                        }
                        let free = free_bits(t);
                        let o = over(p, x[t.v.x].xyz, x[t.v.y].xyz, x[t.v.z].xyz, free);
                        if !o.ok {
                            continue;
                        }
                        let d = o.d * layer_side(own, t);
                        let deepest = select(-RECOVER, 0.0, (free & BORDER) != 0u);
                        if d >= layer_gap(i, t) + MARGIN || d < deepest {
                            continue;
                        }
                        var s = min(n, MAX_TOUCHES - 1u);
                        while s > at {
                            list[s] = list[s - 1u];
                            s--;
                        }
                        list[at] = tk | select(0u, 0x80000000u, d >= 0.0);
                        n = min(n + 1u, MAX_TOUCHES);
                    }
                }
            }
        }
    }
    var kept = 0u;
    for (var s = 0u; s < n; s++) {
        let e = list[s];
        var keep = (e & 0x80000000u) != 0u;
        if !keep {
            let g = triangles[e & 0x7fffffffu].v.w & 0xffffu;
            keep = true;
            for (var q = 0u; q < n; q++) {
                let f = list[q];
                if (f & 0x80000000u) != 0u && (triangles[f & 0x7fffffffu].v.w & 0xffffu) == g {
                    keep = false;
                    break;
                }
            }
        }
        if keep {
            // Bit 31 now: the particle belongs on the side the triangle
            // faces (`layer_solve` needs no garments then).
            let tk = e & 0x7fffffffu;
            let above = layer_side(own, triangles[tk]) > 0.0;
            touches[row + 1u + kept] = tk | select(0u, 0x80000000u, above);
            kept += 1u;
        }
    }
    touches[row] = kept;
}

// Add a push to particle i's sums; one that does not move it (a share of
// 0: pinned, or the inner side resting on the body) is not counted, or it
// would dilute the average of the pushes that do.
fn scatter(i: u32, v: vec3f, w: f32) {
    if w == 0.0 {
        return;
    }
    let q = vec3i(round(v / FIXED));
    atomicAdd(&layer_delta[4u * i], q.x);
    atomicAdd(&layer_delta[4u * i + 1u], q.y);
    atomicAdd(&layer_delta[4u * i + 2u], q.z);
    atomicAdd(&layer_delta[4u * i + 3u], 1);
}

// Inverse mass of i in a contact: 0 if it is the inner side and rests on
// the body (shock propagation, Guendelman et al. 2003).
// With the particles sewn to it that are closed to within OPEN_SEAM, as
// one particle of their summed mass (they move together, see
// `layer_average`).
fn held_w(i: u32, inner: bool) -> f32 {
    return select(welded_w(i, OPEN_SEAM * OPEN_SEAM), 0.0, inner && near[i] != 0u);
}

// Each candidate that came too close: XPBD on side·(p − q)·n ≥ gap (q the
// foot, n held constant), cloth already through coming back at
// RECOVERY_SPEED; then positional friction against the triangle's motion
// this substep. Pushes on the particle and the triangle's corners.
@compute @workgroup_size(WG)
fn layer_solve(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let row = i * (MAX_TOUCHES + 1u);
    let p = x[i].xyz;
    let h = step.gravity_h.w;
    for (var s = 0u; s < touches[row]; s++) {
        let e = touches[row + 1u + s];
        let t = triangles[e & 0x7fffffffu];
        if !holds(t) {
            continue;
        }
        let a = x[t.v.x].xyz;
        let b = x[t.v.y].xyz;
        let c = x[t.v.z].xyz;
        let free = free_bits(t);
        let o = over(p, a, b, c, free);
        if !o.ok {
            continue;
        }
        let side = select(-1.0, 1.0, (e & 0x80000000u) != 0u);
        var depth = layer_gap(i, t) - side * o.d;
        if depth <= 0.0 {
            continue;
        }
        if side * o.d < 0.0 {
            // On which side of the triangle's plane it was at the
            // substep's start.
            let pa = prev[t.v.x].xyz;
            let normal = cross(prev[t.v.y].xyz - pa, prev[t.v.z].xyz - pa);
            if side * dot(prev[i].xyz - pa, normal) < 0.0 {
                if (free & BORDER) != 0u {
                    continue;
                }
                depth = min(depth, RECOVERY_SPEED * h);
            }
        }
        let wp = held_w(i, side < 0.0);
        let w3 = vec3f(held_w(t.v.x, side > 0.0), held_w(t.v.y, side > 0.0), held_w(t.v.z, side > 0.0));
        let denom = wp + dot(o.bary * o.bary, w3);
        if denom <= 1e-12 {
            continue;
        }
        let push = o.n * (side * depth / denom);
        // Friction: the sliding over the triangle this substep (the push,
        // along the normal, does not change it), cancelled up to μ × the
        // push.
        let foot = a * o.bary.x + b * o.bary.y + c * o.bary.z;
        let before = prev[t.v.x].xyz * o.bary.x + prev[t.v.y].xyz * o.bary.y + prev[t.v.z].xyz * o.bary.z;
        let moved = (p - prev[i].xyz) - (foot - before);
        let tangential = moved - o.n * dot(moved, o.n);
        let sliding = length(tangential);
        var k = push;
        if sliding > 0.0 {
            let mu = (pdata[i].z + (pdata[t.v.x].z + pdata[t.v.y].z + pdata[t.v.z].z) / 3.0) * 0.5;
            k -= tangential * (min(mu * depth / sliding, 1.0) / denom);
        }
        scatter(i, k * wp, wp);
        scatter(t.v.x, -k * (o.bary.x * w3.x), o.bary.x * w3.x);
        scatter(t.v.y, -k * (o.bary.y * w3.y), o.bary.y * w3.y);
        scatter(t.v.z, -k * (o.bary.z * w3.z), o.bary.z * w3.z);
    }
}

// Each particle's push from contacts between garments: the average of
// its pushes, over-relaxed by LAYER_RELAXATION (constraint averaging with
// successive over-relaxation, Macklin et al. 2014 §4.2: averaged alone, a
// tucked shirt pressed between the trousers and the body slipped out
// through them over a few substeps, where the CPU's sweep held it), into
// `delta` with the mask of the seam partners closed to within OPEN_SEAM
// (as `self_gather`); `self_apply` then moves each particle with them, so
// a push does not open a seam or a fastened zipper. The sums are cleared.
@compute @workgroup_size(WG)
fn layer_average(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= scene.counts.x {
        return;
    }
    let count = atomicExchange(&layer_delta[4u * i + 3u], 0);
    let q = vec3i(
        atomicExchange(&layer_delta[4u * i], 0),
        atomicExchange(&layer_delta[4u * i + 1u], 0),
        atomicExchange(&layer_delta[4u * i + 2u], 0),
    );
    let p = x[i].xyz;
    var mask = 0u;
    let first = sewn_csr[i];
    for (var k = first; k < sewn_csr[i + 1u] && k - first < 24u; k++) {
        let d = x[sewn_csr[k]].xyz - p;
        if dot(d, d) <= OPEN_SEAM * OPEN_SEAM {
            mask |= 1u << (k - first);
        }
    }
    var push = vec3f(0.0);
    if count > 0 {
        push = vec3f(q) * (FIXED * LAYER_RELAXATION / f32(count));
    }
    delta[i] = vec4f(push, f32(mask));
}
