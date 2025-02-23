struct FragmentInput {
    @location(0) color: vec3<f32>,
};

struct FragmentOutput {
    @location(0) out_color: vec4<f32>,
};

@fragment
fn main(in: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    output.out_color = vec4<f32>(in.color, 1.0);
    return output;
}
