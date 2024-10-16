#version 450

// The triangle vertex positions.
layout(location = 0) in vec2 vert_position;

// The per-instance data.
layout(location = 1) in vec2 position;

layout(set = 0, binding = 0) uniform Camera {
    mat4 proj_mat;
    vec2 cam_pos;
};

void main() {
    gl_Position = vec4(vert_position, 0.0, 1.0) * proj_mat + vec4(position, 0.0, 1.0) * proj_mat + vec4(cam_pos, 0, 1);
}
