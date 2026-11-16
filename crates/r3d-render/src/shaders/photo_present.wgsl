// The path tracer's light times the albedo, exposed, tone-mapped with
// Khronos PBR Neutral and encoded as sRGB.

struct Params {
    size: vec2<u32>,
    samples: u32,
    step: u32,
    exposure: f32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> albedo_sum: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> light: array<vec4<f32>>;

const MIN_ALBEDO: f32 = 0.01;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Khronos PBR Neutral tone mapper (2024).
fn pbr_neutral(color: vec3<f32>) -> vec3<f32> {
    let start = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    var c = color - offset;
    let peak = max(c.r, max(c.g, c.b));
    if peak < start {
        return c;
    }
    let d = 1.0 - start;
    let new_peak = 1.0 - d * d / (peak + d - start);
    c *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(c, vec3<f32>(new_peak), g);
}

fn srgb(c: vec3<f32>) -> vec3<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let lo = x * 12.92;
    let hi = 1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, x <= vec3<f32>(0.0031308));
}

@fragment
fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let x = min(u32(at.x), params.size.x - 1u);
    let y = min(u32(at.y), params.size.y - 1u);
    let i = y * params.size.x + x;
    let n = f32(max(params.samples, 1u));
    let a = max(albedo_sum[i].rgb / n, vec3<f32>(MIN_ALBEDO));
    let c = light[i].rgb * a * params.exposure;
    return vec4<f32>(srgb(pbr_neutral(max(c, vec3<f32>(0.0)))), 1.0);
}
