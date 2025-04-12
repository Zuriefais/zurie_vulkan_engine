@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var t: texture_2d<f32>;

@fragment
fn main(
    @location(0) frag_color: vec4<f32>,
    @location(1) frag_tex_coord: vec2<f32>
) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t, s, frag_tex_coord);
    return frag_color * tex_color;
}
