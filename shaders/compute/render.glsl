#version 450
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
layout(set = 0, binding = 1) buffer GridInBuffer { uint grid_in[]; };
layout(set = 0, binding = 2) buffer GridOutBuffer { uint grid_out[]; };

layout(push_constant) uniform PushConstants {
    vec4 sand_color;
    int step;
} push_constants;

int get_index(ivec2 pos) {
    ivec2 dims = ivec2(imageSize(img));
    return pos.y * dims.x + pos.x;
}

void compute_grid() {
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    int index = get_index(pos);

    ivec2 up_left = pos + ivec2(-1, 1);
    ivec2 up = pos + ivec2(0, 1);
    ivec2 up_right = pos + ivec2(1, 1);
    ivec2 right = pos + ivec2(1, 0);
    ivec2 down_right = pos + ivec2(1, -1);
    ivec2 down = pos + ivec2(0, -1);
    ivec2 down_left = pos + ivec2(-1, -1);
    ivec2 left = pos + ivec2(-1, 0);


    // Dead becomes alive.
    if (grid_in[get_index(up)] == 0) {
        grid_out[index] = 1;
    }
}

void compute_color() {
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    int index = get_index(pos);
    if (grid_out[index] == 1) {
        imageStore(img, pos, push_constants.sand_color);
    } else {
        imageStore(img, pos, vec4(0.0));
    }
}

void main() {
    if (push_constants.step == 0) {
        compute_grid();
    } else {
        compute_color();
    }
}
