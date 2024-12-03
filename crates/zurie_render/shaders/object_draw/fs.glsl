#version 450

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_tex_coord;

layout(set = 0, binding = 1) uniform sampler s;
layout(set = 0, binding = 2) uniform texture2D t;

void main() {
    vec4 tex_color = texture(sampler2D(t, s), frag_tex_coord);
    f_color = tex_color * frag_color;
}
