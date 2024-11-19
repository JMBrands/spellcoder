#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CVoxel {
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
    pub z: ::std::os::raw::c_int,
    pub color: raylib::color::Color,
    pub visible_faces: [::std::os::raw::c_char; 6usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Voxel"][::std::mem::size_of::<CVoxel>() - 24usize];
    ["Alignment of Voxel"][::std::mem::align_of::<CVoxel>() - 4usize];
    ["Offset of field: Voxel::x"][::std::mem::offset_of!(CVoxel, x) - 0usize];
    ["Offset of field: Voxel::y"][::std::mem::offset_of!(CVoxel, y) - 4usize];
    ["Offset of field: Voxel::z"][::std::mem::offset_of!(CVoxel, z) - 8usize];
    ["Offset of field: Voxel::color"][::std::mem::offset_of!(CVoxel, color) - 12usize];
    ["Offset of field: Voxel::visible_faces"]
        [::std::mem::offset_of!(CVoxel, visible_faces) - 16usize];
};
extern "C" {
    pub fn gen_chunk_mesh(voxels: [[[CVoxel; 16usize]; 16usize]; 65535usize]) -> raylib::models::Mesh;
}
