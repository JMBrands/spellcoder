#include "chunkmesh.h"

Mesh GenChunkMesh(Chunk *chunk, unsigned long int seed) {
    Mesh mesh = {0};

    float vertices[] = {
        2.0, 0.0, 0.0,
        1.0, 0.0, 2.0,
        0.0, 1.0, 0.0,
    };

    float normals[] = {
        0.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
    };

    float texcoords[] = {
        0.0, 0.0,
        0.5, 1.0,
        1.0, 0.0,
    };

    mesh.vertices = (float *)RL_MALLOC(3*3*sizeof(float));
    memcpy(mesh.vertices, vertices, 3*3*sizeof(float));

    mesh.normals = (float *)RL_MALLOC(3*3*sizeof(float));
    memcpy(mesh.normals, normals, 3*3*sizeof(float));

    mesh.texcoords = (float *)RL_MALLOC(3*2*sizeof(float));
    memcpy(mesh.texcoords, texcoords, 3*2*sizeof(float));

    mesh.vertexCount = 3;
    mesh.triangleCount = 1;

    UploadMesh(&mesh, 0);
    printf("chunkmesh at %ld, %ld\n", chunk->x, chunk->z);

    return mesh;
}