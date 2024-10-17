#version 450

// The triangle vertex positions.
layout(location = 0) in vec2 vert_position;

// The per-instance data.
layout(location = 1) in vec2 position;
layout(location = 2) in vec2 scale;
layout(location = 3) in vec4 color;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform Camera {
    mat4 proj_mat;
    vec2 cam_pos;
};

void main() {
    gl_Position = vec4(vert_position * scale, 0.0, 1.0) * proj_mat + vec4(position, 0.0, 1.0) * proj_mat + vec4(cam_pos, 0, 1);
    frag_color = color;
}
