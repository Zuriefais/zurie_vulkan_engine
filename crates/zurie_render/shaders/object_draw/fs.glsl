#version 450
layout(location = 0) out vec4 f_color;

// Uniform Block Declaration (without the sampler)
layout(set = 0, binding = 1) uniform UBO {
    vec4 background_color;
};

void main() {
    vec4 color = vec4(1.0, 0.0, 0.0, 1.0);
    f_color = mix(background_color, color, color.a);
}
