use raylib::ffi::{Mesh, Color};

extern "C" {
    pub fn GenChunkMesh(chunk: Chunk, seed: u64) -> Mesh;
}