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
