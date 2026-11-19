// 2D board: flat-coloured triangles in board millimetres.

struct Uniforms {
    transform: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct In {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(v: In) -> Out {
    var o: Out;
    o.pos = u.transform * vec4<f32>(v.pos, 0.0, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_main(i: Out) -> @location(0) vec4<f32> {
    return i.color;
}

// Printed fills: the garment's print (sRGB bytes, shown as they are, like
// the other colours), a tint mixed over it (amount in alpha), and faded
// into the board like the plain fills.
@group(1) @binding(0) var print_texture: texture_2d<f32>;
@group(1) @binding(1) var print_sampler: sampler;

struct PrintIn {
    @location(0) pos: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct PrintOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) tint: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_print(v: PrintIn) -> PrintOut {
    var o: PrintOut;
    o.pos = u.transform * vec4<f32>(v.pos, 0.0, 1.0);
    o.tint = v.tint;
    o.uv = v.uv;
    return o;
}

@fragment
fn fs_print(i: PrintOut) -> @location(0) vec4<f32> {
    let c = textureSample(print_texture, print_sampler, i.uv).rgb;
    return vec4<f32>(mix(c, i.tint.rgb, i.tint.a), 0.6);
}
