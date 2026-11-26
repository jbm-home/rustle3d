//! Contacts between garments worn in layers (docs/physics.md §5, "Several
//! garments"): a particle of one garment is kept on its side of the
//! triangles of another, a few millimetres away. Point–triangle proximity
//! as in Bridson, Fedkiw and Anderson, "Robust Treatment of Collisions,
//! Contact and Friction for Cloth Animation" (2002), solved as XPBD
//! inequality constraints (Macklin et al. 2016) with positional Coulomb
//! friction (Macklin et al. 2014, §6.1).
//!
//! Particle–particle repulsion ([`crate::selfcollide`]) needs spheres as
//! wide as the mesh spacing, or one layer slips between the particles of
//! another: two garments then stay about 2 cm apart, and a shirt cannot
//! be tucked into fitted trousers. A triangle has no gaps, so the layers
//! can lie as close as the cloth is thick.
//!
//! The side a particle must stay on comes from the layer order, not from
//! where it was: garments are numbered from the body out and triangles
//! face out of their garment ([`crate::scene::Triangle`]), so a particle
//! of an outer garment belongs on the side a triangle of an inner one
//! faces, and the other way round. Remembering the side a particle was on
//! at the start of a frame locked in every pass-through: the first frames
//! move the cloth at up to 20 m/s while the seams close, and a garment
//! swept through another stayed through it. Known sides pull such cloth
//! back, from up to [`RECOVER`] deep.
//!
//! A seam still closing leaves a gap between its two pieces that no
//! triangle covers: cloth slipped through it, and the seam then closed
//! over it (a shirt caught outside the trousers at their centre back).
//! Seams are bridged by contact-only triangles between consecutive sewn
//! pairs, used while the seam is open and they face as the cloth they
//! continue: over a closed seam they are slivers facing either way, and
//! one facing in pushed the shirt out through the trousers.
//!
//! Candidates (a particle and a triangle of another garment, close
//! enough to touch) are gathered from a spatial hash (Teschner et al.
//! 2003) once per frame, and again whenever the cloth has moved half of
//! [`MARGIN`] since; every substep pushes a particle that came closer
//! than the contact distance back to its side. Sorted, so results are
//! deterministic.

use std::collections::HashMap;

use glam::Vec3;

use crate::selfcollide::MARGIN;

/// Distance kept between two layers on top of their thicknesses (m), as
/// [`crate::solver::SKIN`] is for the body: the cloth's triangles are
/// flat, and the layers must not show through each other between their
/// vertices.
pub const LAYER_SKIN: f32 = 0.002;
/// A particle touches a triangle it lies over, or not farther past one of
/// its edges than this fraction of the triangle (barycentric): over a
/// crease, a particle near the cloth can lie over neither of the two
/// triangles meeting there. Not past a free edge of the cloth (a waistline,
/// a hem): a shirt bloused over the top of trousers is not inside them.
pub const OVERHANG: f32 = 0.2;
/// A particle this far on the wrong side of another garment's cloth is
/// pulled back through it (m), unless it also lies on the right side of
/// some of that garment's triangles near it (then it is over a fold of
/// that garment turned over, a hem turned up, not through it), or the
/// triangle touches a free edge of the cloth: a garment passes from
/// outside another to inside it around its free edges (a shirt tucked
/// in, bloused over the waistband), and pulled back there, the shirt and
/// the trousers' waist fought without end.
pub const RECOVER: f32 = 0.02;
/// Cloth that was on the wrong side of another garment's at the start of
/// a substep comes back through it at most this fast (m/s); cloth that
/// crossed during the substep is stopped at once, as by any contact.
/// Pulled back from 2 cm deep in one substep, it
/// left at 36 m/s and tore the seams; moved back at once without the
/// speed of the move (pre-stabilisation, as for the body), it dragged
/// its neighbours through instead, and six times more cloth ended on the
/// wrong side.
pub const RECOVERY_SPEED: f32 = 0.5;

/// Near the body, a triangle whose normal points into it more than this
/// (cosine with the body's outward normal) is turned over (the inner
/// sheet of a fold): which side of it another garment belongs on is not
/// known, and it takes no part in contacts between garments. Taken by
/// the layer order, the turned sheet of a shirt's fold at the centre back
/// was pushed out through the trousers, a few millimetres a substep.
pub const TURNED: f32 = -0.5;

/// In [`LayerContact::free`]: the triangle touches a free edge.
pub const BORDER: u8 = 1 << 3;
/// A seam's bridge counts while the seam is open wider than this (m);
/// narrower, the cloth's own triangles cover it ([`OVERHANG`]).
pub const OPEN_SEAM: f32 = 0.005;

/// What the contacts read of each particle.
#[derive(Debug, Clone, Copy)]
pub struct Cloth<'a> {
    /// Inverse masses (1/kg).
    pub w: &'a [f32],
    /// Collision thickness (m).
    pub thickness: &'a [f32],
    pub friction: &'a [f32],
    /// Resting on the body (in the last contact pass with it).
    pub on_body: &'a [bool],
}

/// What contacts between garments need to know of the cloth, fixed for a
/// scene.
#[derive(Debug, Clone, Default)]
pub struct LayerContact {
    /// The garment of each particle.
    pub garment: Vec<u16>,
    /// The cloth's triangles and their garments, then the triangles
    /// bridging seams (contacts only).
    pub triangles: Vec<[u32; 3]>,
    /// How many of `triangles` are the cloth's.
    pub cloth: usize,
    /// Per bridge (triangle `cloth + k`), the cloth triangle it continues.
    pub bridged: Vec<u32>,
    /// For each particle, the others sewn or fastened to it (its seam
    /// group, empty for most particles): those closed to within
    /// [`OPEN_SEAM`] of it move with it when a contact pushes it, as one
    /// particle of their summed mass. Pushed alone, the top of a fly
    /// zipper over a tucked shirt opened by 8 mm.
    pub sewn: Vec<Vec<u32>>,
    pub triangle_garment: Vec<u16>,
    /// Per triangle, its free edges: bit k for the edge opposite corner
    /// k, when no other triangle and no seam continue the cloth there;
    /// bit [`BORDER`] when a corner lies on a free edge.
    pub free: Vec<u8>,
    /// For each particle, the triangles it is a corner of.
    pub incident: Vec<Vec<u32>>,
    /// How far from a particle the corners of a triangle it may touch
    /// within a frame can be (m): the longest edge, stretched, plus the
    /// largest contact distance and [`MARGIN`], or [`RECOVER`]. 0: a
    /// single garment.
    pub reach: f32,
}

/// A particle and a triangle of another garment that may touch this
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Touch {
    pub particle: u32,
    pub triangle: u32,
}

/// Where a point lies over a triangle: its normal (unit), the barycentric
/// weights of the point's foot on it (clamped to the triangle) and the
/// point's signed distance along the normal. `None` for a degenerate
/// triangle, a point farther past an edge than [`OVERHANG`] or past a
/// `free` one (bit k: the edge opposite corner k).
pub fn over(p: Vec3, [a, b, c]: [Vec3; 3], free: u8) -> Option<(Vec3, [f32; 3], f32)> {
    let (e1, e2) = (b - a, c - a);
    let n = e1.cross(e2);
    let area2 = n.length_squared();
    if area2 < 1e-16 {
        return None;
    }
    let n = n / area2.sqrt();
    let rel = p - a;
    // Barycentric coordinates of the foot (Ericson, Real-Time Collision
    // Detection, §3.4).
    let (d00, d01, d11) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));
    let (d20, d21) = (rel.dot(e1), rel.dot(e2));
    let det = d00 * d11 - d01 * d01;
    if det.abs() < 1e-20 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / det;
    let w = (d00 * d21 - d01 * d20) / det;
    let u = 1.0 - v - w;
    let limit = |k: u8| if free & (1 << k) != 0 { 0.0 } else { -OVERHANG };
    if u < limit(0) || v < limit(1) || w < limit(2) {
        return None;
    }
    let (u, v, w) = (u.max(0.0), v.max(0.0), w.max(0.0));
    let sum = u + v + w;
    Some((n, [u / sum, v / sum, w / sum], rel.dot(n)))
}

/// Triangles spanning the gap of each stretch of seam between two
/// consecutive sewn pairs `(a1, b1)`, `(a2, b2)` of `seams` whose sides
/// are mesh edges, two per stretch, wound as the cloth triangle along
/// `a1`–`a2` (returned with each).
fn bridges(seams: &[[u32; 2]], triangles: &[[u32; 3]]) -> Vec<([u32; 3], u32)> {
    // Directed edges of the cloth, and their triangle.
    let mut directed: HashMap<(u32, u32), u32> = HashMap::new();
    for (k, t) in triangles.iter().enumerate() {
        for c in 0..3 {
            directed.insert((t[c], t[(c + 1) % 3]), k as u32);
        }
    }
    let edge = |a: u32, b: u32| directed.contains_key(&(a, b)) || directed.contains_key(&(b, a));
    let mut out = Vec::new();
    for pair in seams.windows(2) {
        let ([a1, b1], [a2, b2]) = (pair[0], pair[1]);
        if a1 == a2 || b1 == b2 || !edge(b1, b2) {
            continue;
        }
        // The cloth runs a1 → a2 in its triangle: the bridge runs the
        // other way along that edge.
        if let Some(k) = directed.get(&(a1, a2)) {
            out.push(([a2, a1, b1], *k));
            out.push(([a2, b1, b2], *k));
        } else if let Some(k) = directed.get(&(a2, a1)) {
            out.push(([a1, a2, b2], *k));
            out.push(([a1, b2, b1], *k));
        }
    }
    out
}

impl LayerContact {
    /// `n` particles; `triangles` with their garments; `seams`: sewn
    /// pairs, in order along each seam; `sewn`: per particle, whether a
    /// seam or a closure holds it to another; `rest_edge`: the longest
    /// edge on the flat pattern (m); `thickness`: the largest collision
    /// thickness (m). Indices are assumed valid.
    pub fn new(
        n: usize,
        triangles: &[[u32; 3]],
        garments: &[u16],
        seams: &[[u32; 2]],
        sewn: &[bool],
        rest_edge: f32,
        thickness: f32,
    ) -> Self {
        let several = garments.windows(2).any(|w| w[0] != w[1]);
        let cloth = triangles.len();
        let mut all: Vec<[u32; 3]> = triangles.to_vec();
        let mut garments = garments.to_vec();
        let mut bridged = Vec::new();
        if several {
            for (t, k) in bridges(seams, triangles) {
                all.push(t);
                garments.push(garments.get(k as usize).copied().unwrap_or(0));
                bridged.push(k);
            }
        }
        let triangles = &all;
        let mut garment = vec![0u16; n];
        let mut incident = vec![Vec::new(); n];
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        let opposite = |t: &[u32; 3], k: usize| {
            let (a, b) = (t[(k + 1) % 3], t[(k + 2) % 3]);
            (a.min(b), a.max(b))
        };
        for (k, (t, g)) in triangles.iter().zip(&garments).enumerate() {
            for v in t {
                garment[*v as usize] = *g;
                incident[*v as usize].push(k as u32);
            }
            for c in 0..3 {
                *edges.entry(opposite(t, c)).or_default() += 1;
            }
        }
        let held = |v: u32| sewn.get(v as usize).copied().unwrap_or(false);
        let mut free: Vec<u8> = triangles
            .iter()
            .map(|t| {
                (0..3).fold(0u8, |bits, c| {
                    let (a, b) = opposite(t, c);
                    let alone = edges.get(&(a, b)).copied().unwrap_or(0) < 2;
                    if alone && !(held(a) && held(b)) {
                        bits | 1 << c
                    } else {
                        bits
                    }
                })
            })
            .collect();
        let mut edge_vertex = vec![false; n];
        for (t, bits) in triangles.iter().zip(&free) {
            for c in 0..3 {
                if bits & (1 << c) != 0 {
                    let (a, b) = opposite(t, c);
                    edge_vertex[a as usize] = true;
                    edge_vertex[b as usize] = true;
                }
            }
        }
        for (t, bits) in triangles.iter().zip(&mut free) {
            if t.iter().any(|v| edge_vertex[*v as usize]) {
                *bits |= BORDER;
            }
        }
        // Cloth stretches a little (and much more while it closes over a
        // gap): a candidate's corner may be farther than the rest edge.
        let reach = if several {
            1.5 * rest_edge + (2.0 * thickness + LAYER_SKIN + MARGIN).max(RECOVER)
        } else {
            0.0
        };
        Self {
            garment,
            triangle_garment: garments,
            triangles: all,
            cloth,
            bridged,
            sewn: Vec::new(),
            free,
            incident,
            reach,
        }
    }

    /// Whether the cloth has moved far enough from `gathered` (where it
    /// was when the candidates were gathered) to need new ones: a
    /// particle and a triangle can then have closed by [`MARGIN`]. The
    /// first frames move the cloth by up to 19 cm while the seams close.
    pub fn moved_on(&self, x: &[Vec3], gathered: &[Vec3]) -> bool {
        let limit = (MARGIN / 2.0) * (MARGIN / 2.0);
        self.reach > 0.0
            && x.iter()
                .zip(gathered)
                .any(|(a, b)| a.distance_squared(*b) > limit)
    }

    /// The distance particle `p` keeps from triangle `t` (m).
    pub fn gap(&self, thickness: &[f32], p: usize, t: [u32; 3]) -> f32 {
        let sheet = t.iter().map(|v| thickness[*v as usize]).sum::<f32>() / 3.0;
        thickness[p] + sheet + LAYER_SKIN
    }

    /// The particles that move with `i` when a contact pushes it: itself
    /// and those sewn to it that are closed to within [`OPEN_SEAM`].
    fn welded<'a>(&'a self, x: &'a [Vec3], i: usize) -> impl Iterator<Item = usize> + 'a {
        let partners = self.sewn.get(i).map(Vec::as_slice).unwrap_or_default();
        std::iter::once(i).chain(
            partners
                .iter()
                .map(|j| *j as usize)
                .filter(move |j| x[*j].distance_squared(x[i]) <= OPEN_SEAM * OPEN_SEAM),
        )
    }

    /// Inverse mass of `i` with what is welded to it (0 if any is pinned).
    fn welded_w(&self, x: &[Vec3], w: &[f32], i: usize) -> f32 {
        let mut mass = 0.0;
        for j in self.welded(x, i) {
            if w[j] == 0.0 {
                return 0.0;
            }
            mass += 1.0 / w[j];
        }
        1.0 / mass
    }

    /// Move `i` and what is welded to it by `delta`.
    fn shift(&self, x: &mut [Vec3], i: usize, delta: Vec3) {
        let partners = self.sewn.get(i).map(Vec::as_slice).unwrap_or_default();
        let at = x[i];
        for j in partners.iter().map(|j| *j as usize) {
            if x[j].distance_squared(at) <= OPEN_SEAM * OPEN_SEAM {
                x[j] += delta;
            }
        }
        x[i] += delta;
    }

    /// Whether triangle `k` holds cloth off now: always for the cloth's
    /// own; a seam's bridge while the seam is open wider than
    /// [`OPEN_SEAM`] and it faces as the cloth it continues.
    fn holds(&self, k: usize, x: &[Vec3]) -> bool {
        let Some(bridge) = k.checked_sub(self.cloth) else {
            return true;
        };
        let (Some(t), Some(r)) = (
            self.triangles.get(k),
            self.bridged
                .get(bridge)
                .and_then(|r| self.triangles.get(*r as usize)),
        ) else {
            return false;
        };
        let [a, b, c] = t.map(|v| x[v as usize]);
        let [ra, rb, rc] = r.map(|v| x[v as usize]);
        let facing = (rb - ra).cross(rc - ra).normalize_or_zero();
        let longest = (b - a).length().max((c - b).length()).max((a - c).length());
        (b - a).cross(c - a).dot(facing) > OPEN_SEAM * longest
    }

    /// +1 if particle `p` belongs on the side triangle `k` faces (its
    /// garment is worn over the triangle's), −1 if under it.
    fn side(&self, p: usize, k: usize) -> f32 {
        if self.garment[p] > self.triangle_garment[k] {
            1.0
        } else {
            -1.0
        }
    }

    /// Gather into `touches` the particles and triangles of other
    /// garments that may touch until a particle has moved
    /// [`MARGIN`] / 2 (see [`Self::moved_on`]).
    /// Which triangles are turned over ([`TURNED`]): `body` gives the
    /// body's outward normal at a point near it (`None` farther away).
    /// Seam bridges never are.
    pub fn turned(&self, x: &[Vec3], body: impl Fn(Vec3) -> Option<Vec3>, out: &mut Vec<bool>) {
        out.clear();
        out.extend(self.triangles.iter().enumerate().map(|(k, t)| {
            if k >= self.cloth {
                return false;
            }
            let [a, b, c] = t.map(|v| x[v as usize]);
            let n = (b - a).cross(c - a).normalize_or_zero();
            body((a + b + c) / 3.0).is_some_and(|g| n.dot(g) < TURNED)
        }));
    }

    /// `turned`: per triangle, whether it is turned over (see
    /// [`Self::turned`]; empty: none is).
    pub fn candidates(
        &self,
        x: &[Vec3],
        w: &[f32],
        thickness: &[f32],
        turned: &[bool],
        touches: &mut Vec<Touch>,
    ) {
        touches.clear();
        if self.reach <= 0.0 {
            return;
        }
        let reach = self.reach;
        let cell = |p: Vec3| (p / reach).floor().as_ivec3().to_array();
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, p) in x.iter().enumerate() {
            grid.entry(cell(*p)).or_default().push(i as u32);
        }
        let mut near: Vec<u32> = Vec::new();
        // This particle's candidates: triangle, its garment, whether the
        // particle is on its side.
        let mut found: Vec<(u32, u16, bool)> = Vec::new();
        for (i, p) in x.iter().enumerate() {
            let own = self.garment[i];
            near.clear();
            let c = cell(*p);
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let Some(list) = grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) else {
                            continue;
                        };
                        for &j in list {
                            let j = j as usize;
                            if self.garment[j] != own && x[j].distance_squared(*p) <= reach * reach
                            {
                                near.extend_from_slice(&self.incident[j]);
                            }
                        }
                    }
                }
            }
            near.sort_unstable();
            near.dedup();
            found.clear();
            for &k in &near {
                let t = self.triangles[k as usize];
                if w[i] + t.iter().map(|v| w[*v as usize]).sum::<f32>() == 0.0
                    || turned.get(k as usize).copied().unwrap_or(false)
                    || !self.holds(k as usize, x)
                {
                    continue;
                }
                let Some((_, _, d)) = over(*p, t.map(|v| x[v as usize]), self.free[k as usize])
                else {
                    continue;
                };
                let d = d * self.side(i, k as usize);
                let deepest = if self.free[k as usize] & BORDER != 0 {
                    0.0
                } else {
                    -RECOVER
                };
                if d < self.gap(thickness, i, t) + MARGIN && d >= deepest {
                    found.push((k, self.triangle_garment[k as usize], d >= 0.0));
                }
            }
            for &(k, g, right) in &found {
                if right || !found.iter().any(|(_, h, r)| *h == g && *r) {
                    touches.push(Touch {
                        particle: i as u32,
                        triangle: k,
                    });
                }
            }
        }
    }

    /// Push the particles of `touches` that came too close back to their
    /// side, with friction against the triangle's motion since `prev`
    /// (the start of the substep, `h` long). Cloth already on the wrong
    /// side then comes back at [`RECOVERY_SPEED`].
    ///
    /// The inner cloth of a contact that rests on the body does not move:
    /// the outer one takes the whole push (shock propagation, Guendelman,
    /// Bridson and Fedkiw 2003). Pushed by its share, a light shirt went
    /// into the body, the body (solved last) pushed it back out, and the
    /// trousers' waist crept through it by a millimetre per substep.
    pub fn solve(&self, touches: &[Touch], x: &mut [Vec3], prev: &[Vec3], cloth: Cloth, h: f32) {
        let Cloth {
            thickness,
            friction,
            ..
        } = cloth;
        for touch in touches {
            let p = touch.particle as usize;
            let Some(&t) = self.triangles.get(touch.triangle as usize) else {
                continue;
            };
            if !self.holds(touch.triangle as usize, x) {
                continue;
            }
            let free = self.free.get(touch.triangle as usize).copied().unwrap_or(0);
            let Some((n, bary, d)) = over(x[p], t.map(|v| x[v as usize]), free) else {
                continue;
            };
            let side = self.side(p, touch.triangle as usize);
            let gap = self.gap(thickness, p, t);
            let mut depth = gap - side * d;
            if depth <= 0.0 {
                continue;
            }
            if side * d < 0.0 {
                // On which side of the triangle's plane it was: a fast
                // particle may then have lain over a neighbour of it.
                let [a, b, c] = t.map(|v| prev[v as usize]);
                let normal = (b - a).cross(c - a);
                if side * (prev[p] - a).dot(normal) < 0.0 {
                    if free & BORDER != 0 {
                        continue;
                    }
                    depth = depth.min(RECOVERY_SPEED * h);
                }
            }
            let corners = t.map(|v| v as usize);
            // Inverse masses (with what is welded to each), the inner
            // side's 0 where it rests on the body.
            let held = |i: usize, inner: bool| {
                if inner && cloth.on_body.get(i).copied().unwrap_or(false) {
                    0.0
                } else {
                    self.welded_w(x, cloth.w, i)
                }
            };
            let wp = held(p, side < 0.0);
            let wc = corners.map(|v| held(v, side > 0.0));
            let denom = wp + wc.iter().zip(bary).map(|(wv, b)| b * b * wv).sum::<f32>();
            if denom <= 1e-12 {
                continue;
            }
            // XPBD on C = side·(p − q)·n − gap ≥ 0, q = Σ bᵢ·cornerᵢ, the
            // normal held constant (Bridson et al. 2002, §7.2), no
            // compliance.
            let push = n * (side * depth / denom);
            self.shift(x, p, push * wp);
            for ((v, wv), b) in corners.iter().zip(wc).zip(bary) {
                self.shift(x, *v, -push * (b * wv));
            }
            // Friction: the sliding of the particle over the triangle
            // this substep, cancelled up to μ × the push.
            let foot = |pos: &[Vec3]| {
                corners
                    .iter()
                    .zip(bary)
                    .map(|(v, b)| pos[*v] * b)
                    .sum::<Vec3>()
            };
            let moved = (x[p] - prev[p]) - (foot(x) - foot(prev));
            let tangential = moved - n * moved.dot(n);
            let sliding = tangential.length();
            if sliding <= 0.0 {
                continue;
            }
            let mu = (friction[p] + corners.iter().map(|v| friction[*v]).sum::<f32>() / 3.0) / 2.0;
            let cancel = tangential * ((mu * depth / sliding).min(1.0) / denom);
            self.shift(x, p, -cancel * wp);
            for ((v, wv), b) in corners.iter().zip(wc).zip(bary) {
                self.shift(x, *v, cancel * (b * wv));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square sheet of `n`×`n` cells of side `cell` at height `y`,
    /// facing up, its particles from `first`.
    fn sheet(n: u32, cell: f32, y: f32, first: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut x = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                x.push(Vec3::new(i as f32 * cell, y, j as f32 * cell));
            }
        }
        let mut t = Vec::new();
        for j in 0..n {
            for i in 0..n {
                let a = first + j * (n + 1) + i;
                let (b, c, d) = (a + 1, a + n + 1, a + n + 2);
                t.push([a, d, b]);
                t.push([a, c, d]);
            }
        }
        (x, t)
    }

    #[test]
    fn a_point_lies_over_a_triangle_or_not() {
        let t = [Vec3::ZERO, Vec3::X, Vec3::Z];
        let (n, b, d) = over(Vec3::new(0.25, 0.1, 0.25), t, 0).unwrap();
        // (b − a) × (c − a) = x × z = −y.
        assert!((n - Vec3::NEG_Y).length() < 1e-6);
        assert!((d + 0.1).abs() < 1e-6);
        assert!((b[0] - 0.5).abs() < 1e-6 && (b[1] - 0.25).abs() < 1e-6);
        assert!(over(Vec3::new(2.0, 0.0, 0.0), t, 0).is_none());
        assert!(over(Vec3::ZERO, [Vec3::ZERO, Vec3::X, Vec3::X * 2.0], 0).is_none());
        // Just past the edge opposite a (x + z = 1): over it, unless the
        // edge is free.
        let past = Vec3::new(0.55, 0.1, 0.55);
        assert!(over(past, t, 0).is_some());
        assert!(over(past, t, 1).is_none());
        assert!(over(past, t, 2).is_some());
    }

    /// Two sheets of 25 mm cells, the lower one (garment 0) at height 0,
    /// the upper one (garment 1) at `y`, offset by half a cell so its
    /// particles lie over the lower one's triangles, not on its
    /// particles; and where the upper one's particles start.
    fn two_sheets(y: f32) -> (Vec<Vec3>, LayerContact, usize) {
        let (mut x, mut tris) = sheet(4, 0.025, 0.0, 0);
        let (upper, upper_tris) = sheet(4, 0.025, y, x.len() as u32);
        let first_upper = x.len();
        x.extend(upper.iter().map(|p| *p + Vec3::new(0.0125, 0.0, 0.0125)));
        let garments: Vec<u16> = (0..tris.len())
            .map(|_| 0)
            .chain(upper_tris.iter().map(|_| 1))
            .collect();
        tris.extend(upper_tris);
        let contact = LayerContact::new(x.len(), &tris, &garments, &[], &[], 0.036, 0.0005);
        (x, contact, first_upper)
    }

    #[test]
    fn a_sheet_pressed_on_another_stays_a_few_millimetres_above_it() {
        // 3 mm apart: far closer than particle spheres allow (20 mm), and
        // between particles too.
        let (mut x, contact, first_upper) = two_sheets(0.003);
        let n = x.len();
        let (w, thickness, friction) = (vec![1.0; n], vec![0.0005; n], vec![0.3; n]);
        let mut touches = Vec::new();
        contact.candidates(&x, &w, &thickness, &[], &mut touches);
        assert!(!touches.is_empty());
        // Press the upper sheet 5 mm down, through the lower one.
        let prev = x.clone();
        for p in &mut x[first_upper..] {
            p.y -= 0.005;
        }
        for _ in 0..20 {
            contact.solve(
                &touches,
                &mut x,
                &prev,
                Cloth {
                    w: &w,
                    thickness: &thickness,
                    friction: &friction,
                    on_body: &[],
                },
                1.0,
            );
        }
        let gap = 0.001 + LAYER_SKIN;
        // Interior particles of the upper sheet (over the lower one) are
        // back above it, at the contact distance.
        for (k, p) in x[first_upper..].iter().enumerate() {
            let (i, j) = (k % 5, k / 5);
            if i < 4 && j < 4 {
                let lower = x[..first_upper].iter().map(|q| q.y).sum::<f32>() / 25.0;
                assert!(p.y - lower > gap - 1e-4, "{k}: {} vs {lower}", p.y);
            }
        }
    }

    #[test]
    fn an_outer_sheet_that_went_through_comes_back_over() {
        // The outer sheet starts 15 mm under the inner one: it is pulled
        // back over it, by the layer order.
        let (mut x, contact, first_upper) = two_sheets(-0.015);
        let n = x.len();
        let (w, thickness, friction) = (vec![1.0; n], vec![0.0005; n], vec![0.0; n]);
        let mut touches = Vec::new();
        for _ in 0..10 {
            contact.candidates(&x, &w, &thickness, &[], &mut touches);
            let prev = x.clone();
            contact.solve(
                &touches,
                &mut x,
                &prev,
                Cloth {
                    w: &w,
                    thickness: &thickness,
                    friction: &friction,
                    on_body: &[],
                },
                1.0,
            );
        }
        let middle = first_upper + 2 * 5 + 1;
        let lower = x[2 * 5 + 2].y;
        assert!(x[middle].y > lower, "{} vs {lower}", x[middle].y);
    }

    /// Solve `touches` `times` times, gathered where `x` was.
    fn solve_from(contact: &LayerContact, x: &mut [Vec3], prev: &[Vec3], on_body: &[bool]) {
        let n = x.len();
        let (w, thickness, friction) = (vec![1.0; n], vec![0.0005; n], vec![0.0; n]);
        let mut touches = Vec::new();
        contact.candidates(prev, &w, &thickness, &[], &mut touches);
        for _ in 0..10 {
            contact.solve(
                &touches,
                x,
                prev,
                Cloth {
                    w: &w,
                    thickness: &thickness,
                    friction: &friction,
                    on_body,
                },
                1.0,
            );
        }
    }

    #[test]
    fn cloth_on_the_body_lets_the_outer_layer_take_the_push() {
        let (mut x, contact, first_upper) = two_sheets(0.003);
        let prev = x.clone();
        for p in &mut x[first_upper..] {
            p.y -= 0.005;
        }
        let on_body: Vec<bool> = (0..x.len()).map(|i| i < first_upper).collect();
        solve_from(&contact, &mut x, &prev, &on_body);
        assert!(x[..first_upper].iter().all(|p| p.y == 0.0));
        let gap = 0.001 + LAYER_SKIN;
        let middle = first_upper + 2 * 5 + 1;
        assert!(x[middle].y > gap - 1e-4, "{}", x[middle].y);
    }

    #[test]
    fn an_open_seam_is_bridged() {
        // The outer garment: two sheets 3 cm apart along x, their facing
        // edges sewn; the inner one: a small sheet under the gap, pushed
        // up through it.
        let (mut x, mut tris) = sheet(2, 0.025, 0.0, 0);
        let first_right = x.len() as u32;
        let (right, right_tris) = sheet(2, 0.025, 0.0, first_right);
        x.extend(right.iter().map(|p| *p + Vec3::X * 0.08));
        tris.extend(right_tris);
        let seams: Vec<[u32; 2]> = (0..3).map(|j| [j * 3 + 2, first_right + j * 3]).collect();
        let first_inner = x.len() as u32;
        let (inner, inner_tris) = sheet(1, 0.01, -0.004, first_inner);
        x.extend(inner.iter().map(|p| *p + Vec3::new(0.06, 0.0, 0.02)));
        let garments: Vec<u16> = tris
            .iter()
            .map(|_| 1)
            .chain(inner_tris.iter().map(|_| 0))
            .collect();
        tris.extend(inner_tris);
        let mut sewn = vec![false; x.len()];
        for [a, b] in &seams {
            sewn[*a as usize] = true;
            sewn[*b as usize] = true;
        }
        let contact = LayerContact::new(x.len(), &tris, &garments, &seams, &sewn, 0.036, 0.0005);
        assert_eq!(contact.triangles.len() - contact.cloth, 4);
        let prev = x.clone();
        for p in &mut x[first_inner as usize..] {
            p.y += 0.008;
        }
        solve_from(&contact, &mut x, &prev, &[]);
        // Each inner particle is back under the bridges it lies under.
        for p in &x[first_inner as usize..] {
            for (k, t) in contact.triangles.iter().enumerate().skip(contact.cloth) {
                let t = t.map(|v| x[v as usize]);
                if let Some((_, _, d)) = over(*p, t, 0) {
                    assert!(d < 0.0, "{p} over bridge {k}: {d}");
                }
            }
        }
        // Closed, the seam's bridges are flat and hold nothing.
        let mut closed = x.clone();
        for [a, b] in &seams {
            closed[*b as usize] = closed[*a as usize];
        }
        assert!((contact.cloth..contact.triangles.len()).all(|k| !contact.holds(k, &closed)));
    }

    #[test]
    fn cloth_turned_over_near_the_body_takes_no_part() {
        // The body below, its outward normal up: the lower sheet faces it
        // the right way; turned over, it is skipped.
        let (x, contact, _) = two_sheets(0.003);
        let up = |_: Vec3| Some(Vec3::Y);
        let mut turned = Vec::new();
        contact.turned(&x, up, &mut turned);
        assert!(turned.iter().all(|t| !t));
        let mut flipped = contact.clone();
        for t in &mut flipped.triangles {
            t.swap(1, 2);
        }
        flipped.turned(&x, up, &mut turned);
        assert!(turned.iter().all(|t| *t));
        let n = x.len();
        let mut touches = Vec::new();
        flipped.candidates(&x, &vec![1.0; n], &vec![0.0005; n], &turned, &mut touches);
        assert!(touches.is_empty());
        // Far from the body nothing is turned.
        flipped.turned(&x, |_| None, &mut turned);
        assert!(turned.iter().all(|t| !t));
    }

    #[test]
    fn one_garment_has_no_layer_contacts() {
        let (x, tris) = sheet(2, 0.025, 0.0, 0);
        let contact = LayerContact::new(
            x.len(),
            &tris,
            &vec![0; tris.len()],
            &[],
            &[],
            0.036,
            0.0005,
        );
        let mut touches = Vec::new();
        contact.candidates(&x, &[1.0; 9], &[0.0005; 9], &[], &mut touches);
        assert!(touches.is_empty());
    }
}
