#version 450
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;

layout(set = 0, binding = 0) uniform Camera {
    mat4 proj_mat;
    vec2 cam_pos;
};

layout(location = 0) out vec2 f_tex_coords;

void main() {
    gl_Position = vec4(position - cam_pos, 0.0, 1.0) * proj_mat;

    f_tex_coords = tex_coords;
}
