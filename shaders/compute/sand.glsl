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

void swapValuesAtomic(uint index1, uint index2) {
    if (index1 >= grid.length() || index2 >= grid.length()) {
        return;
    }
    uint temp = atomicExchange(grid[index1], grid[index2]);
    atomicExchange(grid[index2], temp);
}

void sand(ivec2 pos, ivec2 imgSize) {
    uint cellIndex = get_index(pos);

    ivec2 downPos = pos + ivec2(0, -1);
    int downIndex = get_index(downPos);
    uint downCellValue = grid[downIndex];
    if (downCellValue == EMPTY || downCellValue == WATER) {
        swapValuesAtomic(cellIndex, downIndex);
        return;
    }

    int downLeftIndex = get_index(pos + ivec2(-1, -1));
    int downRightIndex = get_index(pos + ivec2(1, -1));
    bool canMoveDownRight = grid[downRightIndex] == EMPTY || grid[downRightIndex] == WATER;
    bool canMoveDownLeft = grid[downLeftIndex] == EMPTY || grid[downLeftIndex] == WATER;

    if (canMoveDownLeft && canMoveDownRight) {
        if (pos.x % 2 == 0) {

        } else {

        }
    } else if (canMoveDownLeft) {
        swapValuesAtomic(cellIndex, downLeftIndex);
    } else if (canMoveDownRight) {
        swapValuesAtomic(cellIndex, downRightIndex);
    }
}

void water(ivec2 pos, ivec2 imgSize) {
    uint cellIndex = get_index(pos);

    ivec2 downPos = pos + ivec2(0, -1);
    int downIndex = get_index(downPos);
    uint downCellValue = grid[downIndex];
    if (downCellValue == EMPTY) {
        swapValuesAtomic(cellIndex, downIndex);
        return;
    }

    int downLeftIndex = get_index(pos + ivec2(-1, -1));
    int downRightIndex = get_index(pos + ivec2(1, -1));
    bool canMoveDownRight = grid[downRightIndex] == EMPTY;
    bool canMoveDownLeft = grid[downLeftIndex] == EMPTY;

    if (canMoveDownLeft && canMoveDownRight) {
        if (pos.x % 2 == 0) {

        } else {

        }
    } else if (canMoveDownLeft) {
        swapValuesAtomic(cellIndex, downLeftIndex);
        return;
    } else if (canMoveDownRight) {
        swapValuesAtomic(cellIndex, downRightIndex);
        return;
    }

    int leftIndex = get_index(pos + ivec2(-1, 0));
    int rightIndex = get_index(pos + ivec2(1, 0));
    bool canMoveRight = grid[rightIndex] == EMPTY;
    bool canMoveLeft = grid[leftIndex] == EMPTY;

    if (canMoveDownLeft && canMoveDownRight) {
        if (pos.x % 2 == 0) {

        } else {

        }
    } else if (canMoveDownLeft) {
        swapValuesAtomic(cellIndex, downLeftIndex);
        return;
    } else if (canMoveDownRight) {
        swapValuesAtomic(cellIndex, downRightIndex);
        return;
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
