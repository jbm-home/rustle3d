// 3D view: backdrop gradient, ground with contact shadow, and surfaces
// (cloth, avatar) with a soft wrapped diffuse and a sheen term. Colours
// arrive in sRGB; lighting happens in linear space; output is sRGB-encoded.

struct Globals {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    sky_top: vec4<f32>,
    sky_bottom: vec4<f32>,
    ground: vec4<f32>,
};
@group(0) @binding(0) var<uniform> g: Globals;

struct Material {
    base: vec4<f32>,
    sheen: vec4<f32>,
    // x: sheen amount; y: 1 to use the vertex colours (a fit map); z: 1
    // to draw the print (its colours times the base)
    params: vec4<f32>,
};
@group(1) @binding(0) var<uniform> m: Material;
// The surface's print (white without one), repeating, mip-mapped.
@group(2) @binding(0) var print_texture: texture_2d<f32>;
@group(2) @binding(1) var print_sampler: sampler;

fn lin(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

fn srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
}

struct BackdropOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) t: f32,
};

@vertex
fn vs_backdrop(@builtin(vertex_index) i: u32) -> BackdropOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: BackdropOut;
    o.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    o.t = uv.y;
    return o;
}

@fragment
fn fs_backdrop(i: BackdropOut) -> @location(0) vec4<f32> {
    return vec4<f32>(mix(g.sky_bottom.rgb, g.sky_top.rgb, clamp(i.t, 0.0, 1.0)), 1.0);
}

struct GroundOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) xz: vec2<f32>,
};

@vertex
fn vs_ground(@builtin(vertex_index) i: u32) -> GroundOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let c = corners[i] * 3.0;
    var o: GroundOut;
    o.pos = g.view_proj * vec4<f32>(c.x, 0.0, c.y, 1.0);
    o.xz = c;
    return o;
}

@fragment
fn fs_ground(i: GroundOut) -> @location(0) vec4<f32> {
    let d = length(i.xz);
    let fade = 1.0 - smoothstep(0.9, 3.0, d);
    let shadow = 1.0 - 0.28 * (1.0 - smoothstep(0.05, 0.6, d));
    return vec4<f32>(g.ground.rgb * shadow, fade);
}

struct SurfaceIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct SurfaceOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

@vertex
fn vs_surface(v: SurfaceIn) -> SurfaceOut {
    var o: SurfaceOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    o.world = v.pos;
    o.normal = v.normal;
    o.color = v.color;
    o.uv = v.uv;
    return o;
}

@fragment
fn fs_surface(i: SurfaceOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Sampled before any branch (derivatives need uniform control flow).
    let printed = textureSample(print_texture, print_sampler, i.uv).rgb;
    var n = normalize(i.normal);
    if !front {
        n = -n;
    }
    let v = normalize(g.eye.xyz - i.world);
    // Studio lights: key high on the left, soft fill low on the right.
    let key = normalize(vec3<f32>(-0.6, 0.7, 0.4));
    let fill = normalize(vec3<f32>(0.8, 0.2, 0.35));
    let rim = normalize(vec3<f32>(-0.2, 0.4, -0.9));
    var base = select(lin(m.base.rgb), lin(i.color.rgb), m.params.y > 0.5);
    if m.params.z > 0.5 {
        base = base * lin(printed);
    }
    let hemi = mix(lin(g.ground.rgb) * 0.35, lin(g.sky_top.rgb), n.y * 0.5 + 0.5);
    // Slightly wrapped diffuse: fabric scatters a little past the
    // terminator, but folds must keep their shading.
    let wrap = 0.15;
    let kd = max((dot(n, key) + wrap) / (1.0 + wrap), 0.0);
    let fd = max((dot(n, fill) + wrap) / (1.0 + wrap), 0.0);
    let nv = clamp(dot(n, v), 0.0, 1.0);
    // Sheen: fibres catch light at grazing angles.
    let sheen = pow(1.0 - nv, 5.0) * 0.45 + pow(max(dot(n, rim), 0.0), 2.0) * 0.12;
    var c = base * (kd * 1.25 + fd * 0.22 + hemi * 0.18) + lin(m.sheen.rgb) * sheen * m.params.x;
    if !front {
        c = c * 0.55;
    }
    // ACES-like filmic curve (Narkowicz fit).
    c = clamp((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(srgb(c), 1.0);
}
