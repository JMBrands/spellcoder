#include "raylib.h"
#include <stdbool.h>
#include <string.h>
#include <stdlib.h>

typedef enum {
    AIR
} VoxelMaterial;

typedef struct {
    VoxelMaterial material;
    Color *color;
    bool visible_faces[6];
} Voxel;

typedef struct {
    Voxel voxels[16][16][65536];
    long int x;
    long int z;
    Mesh *mesh;
    Model *model;
} Chunk;

Mesh GenChunkMesh(Chunk chunk, unsigned long int seed);