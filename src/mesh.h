#include "raylib/include/raylib.h"
#include <stdbool.h>

typedef enum VoxelMaterial {
    AIR
};

typedef struct Chunk {
    Voxel voxels[16][16][65536];
    long int x;
    long int z;
    Mesh *mesh;
    Model *model
};

typedef struct Voxel {
    VoxelMaterial material;
    Color *color;
    bool visible_faces[6];
};

Mesh GenChunkMesh(Chunk)