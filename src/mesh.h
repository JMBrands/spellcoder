#include <raylib.h>

typedef struct {
    int x;
    int y;
    int z;
    Color color;
    char visible_faces[6]; // Vector with face indices for every face that's visible, the other faces will not be drawn.
                            // 0 = down 1 = up 2 = north 3 = south 5 = east 6 = west
} Voxel;

Mesh gen_chunk_mesh(Voxel*** voxels);