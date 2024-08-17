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

    if (below.y >= imgSize.y || atomicExchange(grid[below.y * imgSize.x + below.x], SAND) == EMPTY) {
        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
    } else if (atomicExchange(grid[below.y * imgSize.x + below.x], SAND) == WATER) {
        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], WATER);
    } else {
        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);
        ivec2 belowRight = pixelCoord + ivec2(1, -1);

        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                (atomicExchange(grid[belowLeft.y * imgSize.x + belowLeft.x], SAND) == EMPTY));
        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                (atomicExchange(grid[belowRight.y * imgSize.x + belowRight.x], SAND) == EMPTY));

        if (canFallLeft && canFallRight) {
            if (gl_GlobalInvocationID.x % 2 == 0) {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            } else {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            }
        } else if (canFallLeft) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
        } else if (canFallRight) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
        } else {
            // Sand stays in place
        }
    }
}

void water(ivec2 pixelCoord, ivec2 imgSize) {
    ivec2 below = pixelCoord + ivec2(0, -1);

    if (below.y >= imgSize.y || atomicExchange(grid[below.y * imgSize.x + below.x], WATER) == EMPTY) {
        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
    } else {
        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);
        ivec2 belowRight = pixelCoord + ivec2(1, -1);

        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                (atomicExchange(grid[belowLeft.y * imgSize.x + belowLeft.x], WATER) == EMPTY));
        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                (atomicExchange(grid[belowRight.y * imgSize.x + belowRight.x], WATER) == EMPTY));

        if (canFallLeft && canFallRight) {
            if (gl_GlobalInvocationID.x % 2 == 0) {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            } else {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            }
        } else if (canFallLeft) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
        } else if (canFallRight) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
        } else {
            // Water stays in place
        }
        ivec2 left = pixelCoord + ivec2(-1, 0);
        ivec2 right = pixelCoord + ivec2(1, 0);

        bool canSlideLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&
                atomicExchange(grid[left.y * imgSize.x + left.x], WATER) == EMPTY);
        bool canSlideRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&
                atomicExchange(grid[right.y * imgSize.x + right.x], WATER) == EMPTY);

        if (canSlideLeft && canSlideRight) {
            if (gl_GlobalInvocationID.x % 2 == 0) {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            } else {
                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
            }
        } else if (canSlideLeft) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
        } else if (canSlideRight) {
            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);
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
        // Wall stays in place
    }
    else {
        // Empty cell stays empty
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
