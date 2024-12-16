use raylib::ffi::{Mesh, Color};

use crate::Chunk;

extern "C" {
    pub fn GenChunkMesh(chunk: &mut Chunk, seed: u64) -> Mesh;
}