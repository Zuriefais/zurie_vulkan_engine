#version 450

layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
layout(set = 0, binding = 1) buffer GridBuffer {
    uint grid[];
};

layout(push_constant) uniform PushConstants {
    vec4[5] palette;
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
#define TAP 4

void swapValuesAtomic(uint index1, uint index2) {
    if (index1 >= grid.length() || index2 >= grid.length()) {
        return;
    }
    uint temp = atomicExchange(grid[index1], grid[index2]);
    atomicExchange(grid[index2], temp);
}

bool rand_bool(){
    // Generate a pseudo-random float between 0.0 and 1.0
    float r = fract(sin(dot(gl_GlobalInvocationID.xy, vec2(12.9898,78.233))) * 43758.5453);

    // Return true if the random value is less than 0.5, otherwise false
    return r < 0.5;
}

bool move_pos1_or_pos2(uint cellIndex, uint li, uint ri, bool l, bool r) {
    if (r && l) {
        if (rand_bool()) {
            swapValuesAtomic(cellIndex, li);
        } else {
            swapValuesAtomic(cellIndex, ri);
        }
    } else if (r) {
        swapValuesAtomic(cellIndex, ri);
        return true;
    } else if (l) {
        swapValuesAtomic(cellIndex, li);
        return true;
    }
    return false;
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
    bool canMoveDownRight = grid[downRightIndex] == EMPTY;
    bool canMoveDownLeft = grid[downLeftIndex] == EMPTY;

    move_pos1_or_pos2(cellIndex, downLeftIndex, downRightIndex, canMoveDownLeft, canMoveDownRight);
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

    if (move_pos1_or_pos2(cellIndex, downLeftIndex, downRightIndex, canMoveDownLeft, canMoveDownRight)) {
        return;
    }

    int leftIndex = get_index(pos + ivec2(-1, 0));
    int rightIndex = get_index(pos + ivec2(1, 0));
    bool canMoveRight = grid[rightIndex] == EMPTY;
    bool canMoveLeft = grid[leftIndex] == EMPTY;

    move_pos1_or_pos2(cellIndex, leftIndex, rightIndex, canMoveLeft, canMoveRight);
}

void tap(ivec2 pos, ivec2 imgSize) {
    uint cellIndex = get_index(pos);
    ivec2 downPos = pos + ivec2(0, -1);
    int downIndex = get_index(downPos);
    uint downCellValue = grid[downIndex];
    if (downCellValue == EMPTY) {
        atomicExchange(grid[downIndex], SAND);
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
    } else if (cellValue == TAP) {
        tap(pixelCoord, imgSize);
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
