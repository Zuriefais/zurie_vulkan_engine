#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
layout(set = 0, binding = 1) buffer GridBuffer { uint grid[]; };

layout(push_constant) uniform PushConstants {
    vec4[4] palette;
} push_constants;

int get_index(ivec2 pos) {
    ivec2 dims = ivec2(imageSize(img));
    return pos.y * dims.x + pos.x;
}

#define EMPTY 0
#define SAND 1
#define WALL 2

void main() {
    ivec2 imgSize = imageSize(img);
    ivec2 pixelCoord = ivec2(gl_GlobalInvocationID.xy);

    if (pixelCoord.x >= imgSize.x || pixelCoord.y >= imgSize.y) {
        return;
    }

    uint cellValue = grid[pixelCoord.y * imgSize.x + pixelCoord.x];

    if (cellValue == SAND) {
        ivec2 below = pixelCoord + ivec2(0, 1);

        if (below.y >= imgSize.y || grid[below.y * imgSize.x + below.x] == EMPTY) {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
            grid[below.y * imgSize.x + below.x] = SAND;
        } else {
            ivec2 belowLeft = pixelCoord + ivec2(-1, 1);
            ivec2 belowRight = pixelCoord + ivec2(1, 1);

            bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                                grid[belowLeft.y * imgSize.x + belowLeft.x] == EMPTY);
            bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                                 grid[belowRight.y * imgSize.x + belowRight.x] == EMPTY);

            if (canFallLeft && canFallRight) {
                if (gl_GlobalInvocationID.x % 2 == 0) {
                    grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                    grid[belowLeft.y * imgSize.x + belowLeft.x] = SAND;
                } else {
                    grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                    grid[belowRight.y * imgSize.x + belowRight.x] = SAND;
                }
            } else if (canFallLeft) {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[belowLeft.y * imgSize.x + belowLeft.x] = SAND;
            } else if (canFallRight) {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[belowRight.y * imgSize.x + belowRight.x] = SAND;
            } else {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = SAND;
            }
        }
    } else if (cellValue == WALL) {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = WALL;
    } else {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
    }

    vec4 color = push_constants.palette[grid[pixelCoord.y * imgSize.x + pixelCoord.x]];
    imageStore(img, pixelCoord, color);
}
