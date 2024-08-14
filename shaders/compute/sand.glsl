#version 450

layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
layout(set = 0, binding = 1) buffer GridBuffer {
    uint grid[];
};

layout(push_constant) uniform PushConstants {
    vec4[4] palette;
    bool simulate;
} push_constants;

int get_index(ivec2 pos) {
    ivec2 dims = ivec2(imageSize(img));
    return pos.y * dims.x + pos.x;
}

#define EMPTY 0
#define SAND 1
#define WALL 2
#define WATER 3

void sand(ivec2 pixelCoord, ivec2 imgSize) {
    ivec2 below = pixelCoord + ivec2(0, -1);

    if (below.y >= imgSize.y || grid[below.y * imgSize.x + below.x] == EMPTY) {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
        grid[below.y * imgSize.x + below.x] = SAND;
    } else if (grid[below.y * imgSize.x + below.x] == WATER) {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = WATER;
        grid[below.y * imgSize.x + below.x] = SAND;
    } else {
        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);
        ivec2 belowRight = pixelCoord + ivec2(1, -1);

        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                (grid[belowLeft.y * imgSize.x + belowLeft.x] == EMPTY));
        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                (grid[belowRight.y * imgSize.x + belowRight.x] == EMPTY));

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
}

void water(ivec2 pixelCoord, ivec2 imgSize) {
    ivec2 below = pixelCoord + ivec2(0, -1);

    if (below.y >= imgSize.y || grid[below.y * imgSize.x + below.x] == EMPTY) {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
        grid[below.y * imgSize.x + below.x] = WATER;
    } else {
        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);
        ivec2 belowRight = pixelCoord + ivec2(1, -1);

        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                grid[belowLeft.y * imgSize.x + belowLeft.x] == EMPTY);
        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                grid[belowRight.y * imgSize.x + belowRight.x] == EMPTY);

        if (canFallLeft && canFallRight) {
            if (gl_GlobalInvocationID.x % 2 == 0) {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[belowLeft.y * imgSize.x + belowLeft.x] = WATER;
            } else {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[belowRight.y * imgSize.x + belowRight.x] = WATER;
            }
        } else if (canFallLeft) {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
            grid[belowLeft.y * imgSize.x + belowLeft.x] = WATER;
        } else if (canFallRight) {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
            grid[belowRight.y * imgSize.x + belowRight.x] = WATER;
        } else {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = WATER;
        }
        ivec2 left = pixelCoord + ivec2(-1, 0);
        ivec2 right = pixelCoord + ivec2(1, 0);

        bool canSlideLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                grid[left.y * imgSize.x + left.x] == EMPTY);
        bool canSlideRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                grid[right.y * imgSize.x + left.x] == EMPTY);

        if (canSlideLeft && canSlideRight) {
            if (gl_GlobalInvocationID.x % 2 == 0) {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[left.y * imgSize.x + left.x] = WATER;
            } else {
                grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
                grid[right.y * imgSize.x + right.x] = WATER;
            }
        } else if (canSlideLeft) {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
            grid[belowLeft.y * imgSize.x + belowLeft.x] = WATER;
        } else if (canSlideRight) {
            grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
            grid[belowRight.y * imgSize.x + belowRight.x] = WATER;
        }
    }
}

void simulate(ivec2 pixelCoord, ivec2 imgSize) {
    if (pixelCoord.x >= imgSize.x || pixelCoord.y >= imgSize.y) {
        return;
    }

    uint cellValue = grid[pixelCoord.y * imgSize.x + pixelCoord.x];

    if (cellValue == SAND) {
        sand(pixelCoord, imgSize);
    } else if (cellValue == WATER) {
        water(pixelCoord, imgSize);
    } else if (cellValue == WALL) {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = WALL;
    }
    else {
        grid[pixelCoord.y * imgSize.x + pixelCoord.x] = EMPTY;
    }
}

void main() {
    ivec2 imgSize = imageSize(img);
    ivec2 pixelCoord = ivec2(gl_GlobalInvocationID.xy);

    if (push_constants.simulate) {
        simulate(pixelCoord, imgSize);
    }
    barrier();

    vec4 color = push_constants.palette[grid[pixelCoord.y * imgSize.x + pixelCoord.x]];
    imageStore(img, pixelCoord, color);
}
