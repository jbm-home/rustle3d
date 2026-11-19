// Progressive path tracer: one sample per pixel per dispatch, added to the
// accumulation buffers. Algorithms and references: photo/mod.rs. The
// constants SHEEN_SIZE and MIN_SHEEN_ALPHA are prepended by the renderer.

const PI: f32 = 3.14159265;
const INV_PI: f32 = 0.31830989;
const FAR: f32 = 1e30;
const NONE: u32 = 0xffffffffu;
// Offset of new rays off a surface (m).
const EPS: f32 = 2e-4;

struct Node {
    lo: vec3<f32>,
    index: u32,
    hi: vec3<f32>,
    count: u32,
};

struct Triangle {
    p0: vec4<f32>, // w: surface index
    p1: vec4<f32>,
    p2: vec4<f32>,
    n0: vec4<f32>,
    n1: vec4<f32>,
    n2: vec4<f32>,
    uv01: vec4<f32>, // corners 0 and 1
    uv2: vec4<f32>,  // corner 2
};

struct Surface {
    base: vec4<f32>,  // linear albedo, translucency
    sheen: vec4<f32>, // sheen colour, sheen roughness
    spec: vec4<f32>,  // roughness, specular strength, print (index + 1; 0: none)
};

struct Light {
    corner: vec4<f32>,   // w: area
    edge_u: vec4<f32>,
    edge_v: vec4<f32>,
    radiance: vec4<f32>, // w: probability of being picked
};

struct Params {
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    size: vec2<u32>,
    sample_index: u32,
    bounces: u32,
    light_count: u32,
    clamp_value: f32,
    pad: vec2<u32>,
    lights: array<Light, 4>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> nodes: array<Node>;
@group(0) @binding(2) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(3) var<storage, read> surfaces: array<Surface>;
// The sheen's albedo table (SHEEN_SIZE² floats, as bits), then the prints:
// their count, each one's offset, width and height, then RGBA8 texels.
@group(0) @binding(4) var<storage, read> sheen_table: array<u32>;
@group(0) @binding(5) var<storage, read_write> color_sum: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> albedo_sum: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read_write> normal_sum: array<vec4<f32>>;

// ---------------------------------------------------------------- random

var<private> rng_state: u32;

// PCG hash (Jarzynski and Olano 2020, "Hash Functions for GPU Rendering").
fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand() -> f32 {
    rng_state = pcg(rng_state);
    return f32(rng_state >> 8u) * (1.0 / 16777216.0);
}

// ---------------------------------------------------------------- geometry

struct Hit {
    t: f32,
    tri: u32,
    u: f32,
    v: f32,
};

fn safe_inverse(d: vec3<f32>) -> vec3<f32> {
    let tiny = vec3<f32>(1e-12);
    let s = select(d, select(tiny, -tiny, d < vec3<f32>(0.0)), abs(d) < tiny);
    return 1.0 / s;
}

// Entry distance into a box, or FAR when missed (or beyond `t_max`).
fn box_entry(lo: vec3<f32>, hi: vec3<f32>, o: vec3<f32>, inv_d: vec3<f32>, t_max: f32) -> f32 {
    let t0 = (lo - o) * inv_d;
    let t1 = (hi - o) * inv_d;
    let near = min(t0, t1);
    let far = max(t0, t1);
    let enter = max(max(near.x, near.y), max(near.z, 0.0));
    let exit = min(min(far.x, far.y), min(far.z, t_max));
    if enter <= exit {
        return enter;
    }
    return FAR;
}

// Closest hit closer than `t_max` (Möller and Trumbore 1997); with `any`,
// the first hit found (shadow rays).
fn intersect(o: vec3<f32>, d: vec3<f32>, t_max: f32, any_hit: bool) -> Hit {
    var hit: Hit;
    hit.t = t_max;
    hit.tri = NONE;
    if arrayLength(&nodes) == 0u {
        return hit;
    }
    let inv_d = safe_inverse(d);
    if box_entry(nodes[0].lo, nodes[0].hi, o, inv_d, t_max) >= FAR {
        return hit;
    }
    var stack: array<u32, 64>;
    var sp = 0u;
    var node = 0u;
    loop {
        let n = nodes[node];
        if n.count > 0u {
            for (var i = 0u; i < n.count; i++) {
                let k = n.index + i;
                let tri = triangles[k];
                let e1 = tri.p1.xyz - tri.p0.xyz;
                let e2 = tri.p2.xyz - tri.p0.xyz;
                let pv = cross(d, e2);
                let det = dot(e1, pv);
                if abs(det) < 1e-14 {
                    continue;
                }
                let inv = 1.0 / det;
                let tv = o - tri.p0.xyz;
                let u = dot(tv, pv) * inv;
                if u < 0.0 || u > 1.0 {
                    continue;
                }
                let qv = cross(tv, e1);
                let v = dot(d, qv) * inv;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let t = dot(e2, qv) * inv;
                if t > 0.0 && t < hit.t {
                    hit.t = t;
                    hit.tri = k;
                    hit.u = u;
                    hit.v = v;
                    if any_hit {
                        return hit;
                    }
                }
            }
        } else {
            let left = node + 1u;
            let right = n.index;
            let tl = box_entry(nodes[left].lo, nodes[left].hi, o, inv_d, hit.t);
            let tr = box_entry(nodes[right].lo, nodes[right].hi, o, inv_d, hit.t);
            if tl < FAR && tr < FAR {
                // Nearer child first, the other on the stack.
                var other = right;
                node = left;
                if tr < tl {
                    node = right;
                    other = left;
                }
                if sp < 64u {
                    stack[sp] = other;
                    sp += 1u;
                }
                continue;
            }
            if tl < FAR {
                node = left;
                continue;
            }
            if tr < FAR {
                node = right;
                continue;
            }
        }
        if sp == 0u {
            break;
        }
        sp -= 1u;
        node = stack[sp];
    }
    return hit;
}

// Nearest light hit by a ray: distance and light index (NONE if none).
// Lights emit on one side only.
fn hit_light(o: vec3<f32>, d: vec3<f32>, t_max: f32) -> vec2<f32> {
    var best = vec2<f32>(t_max, -1.0);
    for (var i = 0u; i < params.light_count; i++) {
        let l = params.lights[i];
        let n = cross(l.edge_u.xyz, l.edge_v.xyz);
        let dn = dot(d, n);
        if dn >= 0.0 {
            continue;
        }
        let t = dot(l.corner.xyz - o, n) / dn;
        if t <= 0.0 || t >= best.x {
            continue;
        }
        let p = o + d * t - l.corner.xyz;
        let a = dot(p, l.edge_u.xyz) / dot(l.edge_u.xyz, l.edge_u.xyz);
        let b = dot(p, l.edge_v.xyz) / dot(l.edge_v.xyz, l.edge_v.xyz);
        if a >= 0.0 && a <= 1.0 && b >= 0.0 && b <= 1.0 {
            best = vec2<f32>(t, f32(i));
        }
    }
    return best;
}

fn sky(d: vec3<f32>) -> vec3<f32> {
    let up = clamp(d.y, 0.0, 1.0);
    return mix(params.sky_horizon.rgb, params.sky_top.rgb, sqrt(up));
}

// Orthonormal basis around n (Duff et al. 2017, "Building an Orthonormal
// Basis, Revisited").
fn basis(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = vec3<f32>(b, s + n.y * n.y * a, -n.y);
    return mat3x3<f32>(t, bt, n);
}

// ---------------------------------------------------------------- BSDF

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn max3(c: vec3<f32>) -> f32 {
    return max(c.x, max(c.y, c.z));
}

fn sheen_at(i: u32) -> f32 {
    return bitcast<f32>(sheen_table[i]);
}

fn sheen_albedo(nv: f32, roughness: f32) -> f32 {
    let size = f32(SHEEN_SIZE - 1u);
    let x = clamp(nv, 0.0, 1.0) * size;
    let y = clamp(roughness, 0.0, 1.0) * size;
    let x0 = u32(floor(x));
    let y0 = u32(floor(y));
    let x1 = min(x0 + 1u, SHEEN_SIZE - 1u);
    let y1 = min(y0 + 1u, SHEEN_SIZE - 1u);
    let fx = x - f32(x0);
    let fy = y - f32(y0);
    let a = mix(sheen_at(y0 * SHEEN_SIZE + x0), sheen_at(y0 * SHEEN_SIZE + x1), fx);
    let b = mix(sheen_at(y1 * SHEEN_SIZE + x0), sheen_at(y1 * SHEEN_SIZE + x1), fx);
    return mix(a, b, fy);
}

fn charlie(alpha: f32, nh: f32) -> f32 {
    let inv = 1.0 / alpha;
    let sin2 = max(1.0 - nh * nh, 1e-7);
    return (2.0 + inv) * pow(sin2, inv * 0.5) / (2.0 * PI);
}

fn sheen_visibility(nl: f32, nv: f32) -> f32 {
    return 1.0 / max(4.0 * (nl + nv - nl * nv), 1e-6);
}

fn ggx_d(a2: f32, nh: f32) -> f32 {
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

fn smith_g1(a2: f32, nx: f32) -> f32 {
    return 2.0 * nx / (nx + sqrt(a2 + (1.0 - a2) * nx * nx));
}

fn schlick(f0: f32, c: f32) -> f32 {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return f0 + (1.0 - f0) * m2 * m2 * m;
}

// The lobes of a surface seen from `nv` (cosine of the view angle).
struct Lobes {
    diffuse: vec3<f32>,  // reflected, Lambertian
    transmit: vec3<f32>, // through the layer, Lambertian
    sheen: vec3<f32>,
    sheen_alpha: f32,
    a2: f32,             // GGX alpha²
    alpha: f32,
    specular: f32,
    p_diffuse: f32,
    p_specular: f32,
    p_transmit: f32,
};

fn lobes(s: Surface, nv: f32) -> Lobes {
    var l: Lobes;
    let tau = s.base.a;
    l.sheen = s.sheen.rgb;
    l.sheen_alpha = max(s.sheen.a * s.sheen.a, MIN_SHEEN_ALPHA);
    l.alpha = max(s.spec.x * s.spec.x, 0.02);
    l.a2 = l.alpha * l.alpha;
    l.specular = s.spec.y;
    // What the sheen reflected, and the specular, the layers below lose.
    let f_spec = schlick(0.04, nv) * l.specular;
    let below = (1.0 - max3(l.sheen) * sheen_albedo(nv, s.sheen.a)) * (1.0 - f_spec);
    l.diffuse = s.base.rgb * (1.0 - tau) * below;
    l.transmit = s.base.rgb * tau * below;
    let wd = luminance(l.diffuse) + 0.5 * luminance(l.sheen) + 1e-4;
    let ws = f_spec;
    let wt = luminance(l.transmit);
    let total = wd + ws + wt;
    l.p_diffuse = wd / total;
    l.p_specular = ws / total;
    l.p_transmit = wt / total;
    return l;
}

// BSDF times |cos θi| (rgb) and the sampling density (a), in the shading
// frame (z = shading normal, wo.z > 0).
fn bsdf_eval(l: Lobes, wo: vec3<f32>, wi: vec3<f32>) -> vec4<f32> {
    let nv = max(wo.z, 1e-4);
    if wi.z > 0.0 {
        let nl = wi.z;
        let h = normalize(wo + wi);
        let nh = max(h.z, 0.0);
        let vh = max(dot(wo, h), 1e-4);
        var f = l.diffuse * INV_PI;
        f += l.sheen * charlie(l.sheen_alpha, nh) * sheen_visibility(nl, nv);
        let d = ggx_d(l.a2, nh);
        let g1v = smith_g1(l.a2, nv);
        f += vec3<f32>(l.specular * schlick(0.04, vh) * d * g1v * smith_g1(l.a2, nl) / (4.0 * nl * nv));
        let pdf = l.p_diffuse * nl * INV_PI + l.p_specular * g1v * d / (4.0 * nv);
        return vec4<f32>(f * nl, pdf);
    }
    let nl = -wi.z;
    return vec4<f32>(l.transmit * INV_PI * nl, l.p_transmit * nl * INV_PI);
}

fn cosine_sample() -> vec3<f32> {
    let r = sqrt(rand());
    let phi = 2.0 * PI * rand();
    return vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - r * r)));
}

// A GGX normal visible from wo (Heitz 2018).
fn visible_normal(wo: vec3<f32>, alpha: f32) -> vec3<f32> {
    let vh = normalize(vec3<f32>(alpha * wo.x, alpha * wo.y, wo.z));
    let lensq = vh.x * vh.x + vh.y * vh.y;
    var t1 = vec3<f32>(1.0, 0.0, 0.0);
    if lensq > 0.0 {
        t1 = vec3<f32>(-vh.y, vh.x, 0.0) / sqrt(lensq);
    }
    let t2 = cross(vh, t1);
    let r = sqrt(rand());
    let phi = 2.0 * PI * rand();
    let p1 = r * cos(phi);
    let s = 0.5 * (1.0 + vh.z);
    let p2 = (1.0 - s) * sqrt(max(0.0, 1.0 - p1 * p1)) + s * r * sin(phi);
    let nh = p1 * t1 + p2 * t2 + sqrt(max(0.0, 1.0 - p1 * p1 - p2 * p2)) * vh;
    return normalize(vec3<f32>(alpha * nh.x, alpha * nh.y, max(1e-6, nh.z)));
}

fn bsdf_sample(l: Lobes, wo: vec3<f32>) -> vec3<f32> {
    let pick = rand();
    if pick < l.p_diffuse {
        return cosine_sample();
    }
    if pick < l.p_diffuse + l.p_specular {
        let h = visible_normal(wo, l.alpha);
        return reflect(-wo, h);
    }
    let c = cosine_sample();
    return vec3<f32>(c.xy, -c.z);
}

fn power_heuristic(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    return a2 / max(a2 + b * b, 1e-20);
}

fn finite(c: vec3<f32>) -> vec3<f32> {
    if all(c >= vec3<f32>(0.0)) && all(c < vec3<f32>(1e20)) {
        return c;
    }
    return vec3<f32>(0.0);
}

fn clamp_contribution(c: vec3<f32>) -> vec3<f32> {
    let m = max3(c);
    if m > params.clamp_value {
        return c * (params.clamp_value / m);
    }
    return c;
}

// ---------------------------------------------------------------- tracing

// Where rays leave a hit point: lifted onto the curved surface the vertex
// normals describe (Hanika 2021), so shadows of coarse triangles do not
// show their facets. `side` flips the normals of a surface seen from
// behind.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn texel(base: u32, w: u32, h: u32, x: i32, y: i32) -> vec3<f32> {
    let xi = u32(((x % i32(w)) + i32(w)) % i32(w));
    let yi = u32(((y % i32(h)) + i32(h)) % i32(h));
    return srgb_to_linear(unpack4x8unorm(sheen_table[base + yi * w + xi]).rgb);
}

// The colour of print `index` (from 1) at `uv` (in repeats): bilinear,
// repeating, in linear light.
fn print_color(index: u32, uv: vec2<f32>) -> vec3<f32> {
    let head = SHEEN_SIZE * SHEEN_SIZE;
    let k = head + 1u + 3u * (index - 1u);
    let base = head + sheen_table[k];
    let w = max(sheen_table[k + 1u], 1u);
    let h = max(sheen_table[k + 2u], 1u);
    let p = (uv - floor(uv)) * vec2<f32>(f32(w), f32(h)) - 0.5;
    let f = fract(p);
    let x = i32(floor(p.x));
    let y = i32(floor(p.y));
    let top = mix(texel(base, w, h, x, y), texel(base, w, h, x + 1, y), f.x);
    let bottom = mix(texel(base, w, h, x, y + 1), texel(base, w, h, x + 1, y + 1), f.x);
    return mix(top, bottom, f.y);
}

fn lifted(tri: Triangle, b: vec3<f32>, p: vec3<f32>, side: f32) -> vec3<f32> {
    let n0 = tri.n0.xyz * side;
    let n1 = tri.n1.xyz * side;
    let n2 = tri.n2.xyz * side;
    let d0 = max(0.0, -dot(p - tri.p0.xyz, n0));
    let d1 = max(0.0, -dot(p - tri.p1.xyz, n1));
    let d2 = max(0.0, -dot(p - tri.p2.xyz, n2));
    return p + b.x * d0 * n0 + b.y * d1 * n1 + b.z * d2 * n2;
}

@compute @workgroup_size(8, 8)
fn trace(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.size.x || id.y >= params.size.y {
        return;
    }
    let pixel = id.y * params.size.x + id.x;
    rng_state = pcg(pixel + pcg(params.sample_index + 0x9e3779b9u));

    // A ray through a random point of the pixel.
    let size = vec2<f32>(params.size);
    let at = vec2<f32>(f32(id.x) + rand(), f32(id.y) + rand());
    let ndc = vec2<f32>(2.0 * at.x / size.x - 1.0, 1.0 - 2.0 * at.y / size.y);
    let near = params.inv_view_proj * vec4<f32>(ndc, 0.0, 1.0);
    let far = params.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    var o = params.eye.xyz;
    var d = normalize(far.xyz / far.w - near.xyz / near.w);

    var radiance = vec3<f32>(0.0);
    var throughput = vec3<f32>(1.0);
    // What the first hit tells the denoiser.
    var first_albedo = vec3<f32>(1.0);
    var first_normal = vec3<f32>(0.0);
    var first_depth = 1e4;
    var first_hit = 0.0;
    // Density of the last BSDF sample (0: the camera ray).
    var last_pdf = 0.0;

    for (var bounce = 0u; bounce <= params.bounces; bounce++) {
        let hit = intersect(o, d, FAR, false);
        // Soft boxes light the scene and show in reflections, but the
        // camera does not see them (they stand out of frame in a studio).
        var light = vec2<f32>(hit.t, -1.0);
        if bounce > 0u {
            light = hit_light(o, d, hit.t);
        }
        if light.y >= 0.0 {
            let l = params.lights[u32(light.y)];
            var w = 1.0;
            if last_pdf > 0.0 {
                let n = normalize(cross(l.edge_u.xyz, l.edge_v.xyz));
                let cos_l = max(-dot(d, n), 1e-6);
                let light_pdf = l.radiance.w * light.x * light.x / (l.corner.w * cos_l);
                w = power_heuristic(last_pdf, light_pdf);
            }
            var c = throughput * l.radiance.rgb * w;
            if bounce > 1u {
                c = clamp_contribution(c);
            }
            radiance += c;
            break;
        }
        if hit.tri == NONE {
            var c = throughput * sky(d);
            if bounce > 1u {
                c = clamp_contribution(c);
            }
            radiance += c;
            break;
        }

        let tri = triangles[hit.tri];
        let b = vec3<f32>(1.0 - hit.u - hit.v, hit.u, hit.v);
        let p = tri.p0.xyz * b.x + tri.p1.xyz * b.y + tri.p2.xyz * b.z;
        let wo_world = -d;
        // Cloth is seen from both sides: turn the normals to the viewer.
        var ng = normalize(cross(tri.p1.xyz - tri.p0.xyz, tri.p2.xyz - tri.p0.xyz));
        var ns = normalize(tri.n0.xyz * b.x + tri.n1.xyz * b.y + tri.n2.xyz * b.z);
        var side = 1.0;
        if dot(ns, wo_world) < 0.0 {
            side = -1.0;
            ns = -ns;
        }
        if dot(ng, ns) < 0.0 {
            ng = -ng;
        }
        if dot(ng, wo_world) < 0.0 {
            // Seen past a silhouette: shade with the true face.
            ng = -ng;
            ns = ng;
        }
        var s = surfaces[u32(tri.p0.w)];
        let print = u32(s.spec.z);
        if print > 0u {
            let uv = tri.uv01.xy * b.x + tri.uv01.zw * b.y + tri.uv2.xy * b.z;
            s.base = vec4<f32>(print_color(print, uv), s.base.a);
        }
        let frame = basis(ns);
        let to_local = transpose(frame);
        let wo = to_local * wo_world;
        let lb = lobes(s, wo.z);
        if bounce == 0u {
            first_albedo = s.base.rgb;
            first_normal = ns;
            first_depth = hit.t;
            first_hit = 1.0;
        }
        let out_front = lifted(tri, b, p, side) + ng * EPS;
        let out_back = p - ng * EPS;

        // Next event estimation: a point on one soft box.
        if params.light_count > 0u {
            let r = rand();
            var pick = 0u;
            var acc = 0.0;
            for (var i = 0u; i < params.light_count; i++) {
                pick = i;
                acc += params.lights[i].radiance.w;
                if r < acc {
                    break;
                }
            }
            let l = params.lights[pick];
            let q = l.corner.xyz + l.edge_u.xyz * rand() + l.edge_v.xyz * rand();
            let front = dot(q - p, ng) > 0.0;
            let origin = select(out_back, out_front, front);
            let to = q - origin;
            let dist = length(to);
            let wi_world = to / max(dist, 1e-9);
            let n_l = normalize(cross(l.edge_u.xyz, l.edge_v.xyz));
            let cos_l = -dot(wi_world, n_l);
            let wi = to_local * wi_world;
            if cos_l > 1e-6 && (wi.z > 0.0) == front {
                let e = bsdf_eval(lb, wo, wi);
                if max3(e.rgb) > 0.0 {
                    let shadow = intersect(origin, wi_world, dist * (1.0 - 1e-4), true);
                    if shadow.tri == NONE {
                        let light_pdf = l.radiance.w * dist * dist / (l.corner.w * cos_l);
                        let w = power_heuristic(light_pdf, e.a);
                        var c = throughput * l.radiance.rgb * e.rgb * (w / light_pdf);
                        if bounce > 0u {
                            c = clamp_contribution(c);
                        }
                        radiance += c;
                    }
                }
            }
        }

        // Continue the path in a direction the BSDF picks.
        let wi = bsdf_sample(lb, wo);
        let e = bsdf_eval(lb, wo, wi);
        if e.a <= 1e-8 || max3(e.rgb) <= 0.0 {
            break;
        }
        let wi_world = frame * wi;
        let front = dot(wi_world, ng) > 0.0;
        if front != (wi.z > 0.0) {
            break;
        }
        throughput *= e.rgb / e.a;
        last_pdf = e.a;
        o = select(out_back, out_front, front);
        d = wi_world;
        if bounce >= 3u {
            let keep = clamp(max3(throughput), 0.05, 0.95);
            if rand() > keep {
                break;
            }
            throughput /= keep;
        }
    }

    let c = finite(radiance);
    let lum = luminance(c);
    let sample = vec4<f32>(c, lum * lum);
    let aux = vec4<f32>(first_albedo, first_depth);
    let nrm = vec4<f32>(first_normal, first_hit);
    if params.sample_index == 0u {
        color_sum[pixel] = sample;
        albedo_sum[pixel] = aux;
        normal_sum[pixel] = nrm;
    } else {
        color_sum[pixel] += sample;
        albedo_sum[pixel] += aux;
        normal_sum[pixel] += nrm;
    }
}
