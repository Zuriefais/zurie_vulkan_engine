#version 450
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 instance_position;

layout(set = 0, binding = 0) uniform Camera {
    mat4 proj_mat;
    vec2 cam_pos;
};

void main() {
    gl_Position = vec4((position + instance_position - cam_pos), 0.0, 1.0) * proj_mat;
}
