#version 450
layout(location = 0) in vec2 v_tex_coords;
layout(location = 0) out vec4 f_color;

// Uniform Block Declaration (without the sampler)
layout(set = 0, binding = 1) uniform UBO {
    vec4 background_color;
};

// Sampler Declaration with its own binding
layout(set = 0, binding = 2) uniform sampler s;

layout(set = 0, binding = 3) uniform texture2D tex;

void main() {
    vec4 color = texture(sampler2D(tex, s), v_tex_coords);
    f_color = mix(background_color, color, color.a);
}
