struct Camera {
    proj_mat: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexInput {
    @location(0) vert_position: vec2<f32>,
};

struct InstanceInput {
    @location(1) position: vec2<f32>,
    @location(2) scale: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
    @location(1) frag_tex_coord: vec2<f32>,
};

@vertex
fn main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let scaled_pos = in.vert_position * inst.scale;
    let world_pos = scaled_pos + inst.position;
    out.position = camera.proj_mat * vec4<f32>(world_pos, 0.0, 1.0);
    out.frag_color = inst.color;
    out.frag_tex_coord = vec2<f32>(in.vert_position.x + 0.5, in.vert_position.y + 0.5);
    return out;
}
