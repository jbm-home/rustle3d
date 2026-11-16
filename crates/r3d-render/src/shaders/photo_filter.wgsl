// Denoiser of the path tracer: an edge-avoiding à-trous wavelet filter
// (Dammertz et al. 2010) with the luminance-variance edge stopping of SVGF
// (Schied et al. 2017), on demodulated light (colour / albedo).
// `prepare` turns the sums into light and variance; each `atrous` pass
// filters with taps `step` pixels apart (1, 2, 4, 8, 16).

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
@group(0) @binding(1) var<storage, read> color_sum: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> albedo_sum: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normal_sum: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> src: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> dst: array<vec4<f32>>;

// Darkest albedo divided by (keeps black fabric from blowing up noise).
const MIN_ALBEDO: f32 = 0.01;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8)
fn prepare(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.size.x || id.y >= params.size.y {
        return;
    }
    let i = id.y * params.size.x + id.x;
    let n = f32(max(params.samples, 1u));
    let c = color_sum[i].rgb / n;
    let m2 = color_sum[i].a / n;
    let l = luminance(c);
    // Variance of the mean of n samples.
    let variance = max(m2 - l * l, 0.0) / n;
    let a = max(albedo_sum[i].rgb / n, vec3<f32>(MIN_ALBEDO));
    let la = max(luminance(a), MIN_ALBEDO);
    dst[i] = vec4<f32>(c / a, variance / (la * la));
}

@compute @workgroup_size(8, 8)
fn atrous(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.size.x || id.y >= params.size.y {
        return;
    }
    let w = params.size.x;
    let h = params.size.y;
    let i = id.y * w + id.x;
    let n = f32(max(params.samples, 1u));
    let centre = src[i];
    let lp = luminance(centre.rgb);
    let np = normal_sum[i].xyz / n;
    let hp = normal_sum[i].w / n;
    let zp = albedo_sum[i].a / n;
    // The variance, blurred over 3 × 3 pixels: a few samples estimate it
    // poorly (SVGF).
    var blurred = 0.0;
    var taps = 0.0;
    for (var by = -1; by <= 1; by++) {
        for (var bx = -1; bx <= 1; bx++) {
            let x = i32(id.x) + bx;
            let y = i32(id.y) + by;
            if x >= 0 && y >= 0 && x < i32(w) && y < i32(h) {
                let k = select(0.5, 1.0, bx == 0) * select(0.5, 1.0, by == 0);
                blurred += src[u32(y) * w + u32(x)].a * k;
                taps += k;
            }
        }
    }
    let sigma_l = 4.0 * sqrt(max(blurred / taps, 0.0)) + 1e-4;
    var kernel = array<f32, 5>(0.0625, 0.25, 0.375, 0.25, 0.0625);
    var sum = vec3<f32>(0.0);
    var weights = 0.0;
    var variance = 0.0;
    for (var dy = -2; dy <= 2; dy++) {
        for (var dx = -2; dx <= 2; dx++) {
            let x = i32(id.x) + dx * i32(params.step);
            let y = i32(id.y) + dy * i32(params.step);
            if x < 0 || y < 0 || x >= i32(w) || y >= i32(h) {
                continue;
            }
            let j = u32(y) * w + u32(x);
            let q = src[j];
            let hq = normal_sum[j].w / n;
            var wn = 0.0;
            if hp > 0.5 && hq > 0.5 {
                let nq = normal_sum[j].xyz / n;
                wn = pow(max(dot(normalize(np), normalize(nq)), 0.0), 32.0);
            } else if hp <= 0.5 && hq <= 0.5 {
                wn = 1.0;
            }
            let zq = albedo_sum[j].a / n;
            let reach = f32(params.step) * length(vec2<f32>(f32(dx), f32(dy)));
            let wz = exp(-abs(zp - zq) / (0.01 * zp * reach + 1e-3));
            let wl = exp(-abs(lp - luminance(q.rgb)) / sigma_l);
            let wk = kernel[dx + 2] * kernel[dy + 2] * wn * wz * wl;
            sum += q.rgb * wk;
            weights += wk;
            variance += q.a * wk * wk;
        }
    }
    if weights <= 1e-12 {
        dst[i] = centre;
        return;
    }
    dst[i] = vec4<f32>(sum / weights, variance / (weights * weights));
}
