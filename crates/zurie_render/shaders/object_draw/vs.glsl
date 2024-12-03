#version 450

layout(location = 0) in vec2 vert_position;

layout(location = 1) in vec2 position;
layout(location = 2) in vec2 scale;
layout(location = 3) in vec4 color;

layout(location = 0) out vec4 frag_color;
layout(location = 1) out vec2 frag_tex_coord;

layout(set = 0, binding = 0) uniform Camera {
    mat4 proj_mat;
    vec2 cam_pos;
};

void main() {
    vec2 world_pos = (vert_position * scale) + position - cam_pos;
    gl_Position = vec4(world_pos, 0.0, 1.0) * proj_mat;
    frag_color = color;
    // Convert vertex position to texture coordinates
    frag_tex_coord = vec2(
        vert_position.x + 0.5,  // Convert from [-0.5, 0.5] to [0, 1]
        1.0 - (vert_position.y + 0.5)  // Flip Y and convert to [0, 1]
    );
}
