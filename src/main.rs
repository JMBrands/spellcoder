use ::core::time;
use std::{
    cmp::Ordering, f32::consts::PI, fmt::{self, Debug}, mem, os::raw::c_int, ptr::null, thread
};

use raylib::prelude::*;
use worldgen::noise::{perlin::PerlinNoise, NoiseProvider};

const SPEED: f32 = 32.0;

#[derive(Clone, Copy)]
#[repr(C)]
enum VoxelMaterial {
    AIR
}

struct Player {
    position: Vector3,
    camera: Camera3D,
    size: Vector3,
}
impl Player {
    fn new(position: Vector3) -> Self {
        let player = Player {
            position,
            camera: Camera3D::perspective(
                Vector3 {
                    x: position.x + 8.0,
                    y: position.y + 2.40,
                    z: position.z + 8.0,
                },
                Vector3 {
                    x: position.x + 9.0,
                    y: position.y + 24.0,
                    z: position.z + 8.0,
                } * Vector3::zero(),
                Vector3::up(),
                70.0,
            ),
            size: Vector3 {
                x: 16.0,
                y: 32.0,
                z: 16.0,
            },
        };
        // player.set_look_direction_vec2(Vector2 {
        //     x: 0.0,
        //     y: -PI / 2.0,
        // });
        player
    }
    // get normalized vector of the direction of the camera
    fn get_look_direction(&self) -> Vector3 {
        (self.camera.target - self.camera.position).normalized()
    }

    // set direction of the camera
    fn set_look_direction(&mut self, dir: Vector3) {
        self.camera.target = self.camera.position + dir;
    }

    // get direction of the camera as yaw & pitch
    fn get_look_direction_vec2(&self) -> Vector2 {
        let vec3 = self.get_look_direction();
        let h_vec2 = Vector2 {
            x: vec3.x,
            y: vec3.z,
        }
        .normalized();
        Vector2 {
            x: h_vec2.y.acos() * h_vec2.x.signum(),
            y: vec3.y.asin(),
        }
    }

    // set direction of the camera as yaw & pitch
    fn set_look_direction_vec2(&mut self, dir: Vector2) {
        let h_vec2 = Vector2 {
            x: dir.x.sin(),
            y: dir.x.cos(),
        }
        .normalized()
            * dir.y.cos().abs();
        self.set_look_direction(
            Vector3 {
                x: h_vec2.x,
                y: dir.y.sin(),
                z: h_vec2.y,
            }
            .normalized(),
        );
    }

    // move camera without changing yaw & pitch
    fn move_camera(&mut self, delta: Vector3) {
        self.position += delta;
        self.camera.position += delta;
        self.camera.target += delta;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Voxel {
    material: VoxelMaterial,
    color: ffi::Color,
    visible_faces: [bool; 6], // Vector with face indices for every face that's visible, the other faces will not be drawn.
                            // 0 = down 1 = up 2 = north 3 = south 4 = east 5 = west
}
impl Debug for Voxel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Voxel")
            .field("color", &self.color)
            .field("visible_faces", &self.visible_faces)
            .finish()
    }
}

trait VoxelDraw {
    fn draw_chunk(&mut self, chunk: &Chunk);
    fn draw_world(&mut self, world: &World);
}
impl VoxelDraw for RaylibMode3D<'_, RaylibDrawHandle<'_>> {
    fn draw_chunk(&mut self, chunk: &Chunk) {
        self.draw_model(&chunk.model, Vector3{x: chunk.x as f32, y: 0.0, z: chunk.z as f32}, 2.0, Color::WHITE);
    }
    fn draw_world(&mut self, world: &World) {
        for chunk in &world.chunks {
            self.draw_chunk(chunk);
        }
    }
}

#[repr(C)]
struct Chunk {
    voxels: [[[Voxel; 16]; 16]; u16::MAX as usize + 1],
    x: i64,
    z: i64,
    mesh: ffi::Mesh,
    model: Model
}
impl Chunk {
    fn new(rl: &mut RaylibHandle, x: i64, z: i64, thread: &RaylibThread) -> Chunk {
            let mut voxels: [[[Voxel; 16]; 16]; u16::MAX as usize + 1] = [[[Voxel {material: VoxelMaterial::AIR, color: Color::WHITE.into(), visible_faces: [true; 6]}; 16]; 16]; 65536];

        let mesh = unsafe {ffi::GenMeshCube(1.0, 1.0, 1.0)};
        Chunk {
            voxels,
            x,
            z,
            mesh,
            model: rl.load_model_from_mesh(thread, unsafe { WeakMesh::from_raw(mesh) }).expect("Error loading mesh")
        }
    }

    fn generate(
        rl: &mut RaylibHandle,
        chunk_x: i64,
        chunk_z: i64,
        noise: &PerlinNoise,
        seed: u64,
        thread: &RaylibThread,
    ) -> Self {
        let mut chunk = Chunk::new(rl, chunk_x * 16, chunk_z * 16, thread);
        for x in 0..16 {
            for z in 0..16 {
                chunk.add_voxel(Voxel {
                    x: chunk_x * 16 + x,
                    y: ((noise.generate(
                        (chunk_x * 16 + x) as f64 / 48.0,
                        (chunk_z * 16 + z) as f64 / 48.0,
                        seed,
                    ) + 1.0)
                        * 8.0) as u16
                        + ((noise.generate(
                            (chunk_x * 16 + x) as f64 / 128.0,
                            (chunk_z * 16 + z) as f64 / 128.0,
                            seed / 2,
                        ) + 1.0)
                            * 128.0) as u16,
                    z: chunk_z * 16 + z,
                    color: Color {
                        r: (x * 16) as u8,
                        g: 255,
                        b: (z * 16) as u8,
                        a: 255,
                    }
                    .into(),
                    visible_faces: [false,true,true,true,true,true]
                });
                // println!("{}", noise.generate((chunk_x * 16 + x) as f64 / 32.0, (chunk_z * 16 + z) as f64 / 32.0, seed));
            }
        }
        chunk.gen_mesh(rl, thread, noise, seed);
        chunk
    }


    fn gen_mesh(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        noise: &PerlinNoise,
        seed: u64,
    )  {
        unsafe {
            /*
            pub struct Mesh {
                pub vertexCount: ::std::os::raw::c_int,
                pub triangleCount: ::std::os::raw::c_int,
                pub vertices: *mut f32,
                pub texcoords: *mut f32,
                pub texcoords2: *mut f32,
                pub normals: *mut f32,
                pub tangents: *mut f32,
                pub colors: *mut ::std::os::raw::c_uchar,
                pub indices: *mut ::std::os::raw::c_ushort,
                pub animVertices: *mut f32,
                pub animNormals: *mut f32,
                pub boneIds: *mut ::std::os::raw::c_uchar,
                pub boneWeights: *mut f32,
                pub vaoId: ::std::os::raw::c_uint,
                pub vboId: *mut ::std::os::raw::c_uint,
            }
            */
            let triangle_count: c_int = 1;
            let vertex_count: c_int = triangle_count * 3;
            let vertices: *mut f32 = libc::malloc(mem::size_of::<f32>() * 3 * vertex_count as usize) as *mut f32;
            let texcoords: *mut f32 = libc::malloc(mem::size_of::<f32>() * 2 * vertex_count as usize) as *mut f32;
            let normals: *mut f32 = libc::malloc(mem::size_of::<f32>() * 3 * vertex_count as usize) as *mut f32;

            *vertices.wrapping_add(0) = 2.0;
            *vertices.wrapping_add(1) = 0.0;
            *vertices.wrapping_add(2) = 0.0;
            *normals.wrapping_add(0) = 0.0;
            *normals.wrapping_add(1) = 1.0;
            *normals.wrapping_add(2) = 0.0;
            *texcoords.wrapping_add(0) = 0.0;
            *texcoords.wrapping_add(1) = 0.0;

            *vertices.wrapping_add(3) = 1.0;
            *vertices.wrapping_add(4) = 0.0;
            *vertices.wrapping_add(5) = 2.0;
            *normals.wrapping_add(3) = 0.0;
            *normals.wrapping_add(4) = 1.0;
            *normals.wrapping_add(6) = 0.0;
            *texcoords.wrapping_add(2) = 0.5;
            *texcoords.wrapping_add(3) = 1.0;

            *vertices.wrapping_add(6) = 0.0;
            *vertices.wrapping_add(7) = 1.0;
            *vertices.wrapping_add(8) = 0.0;
            *normals.wrapping_add(6) = 0.0;
            *normals.wrapping_add(7) = 1.0;
            *normals.wrapping_add(8) = 0.0;
            *texcoords.wrapping_add(4) = 1.0;
            *texcoords.wrapping_add(5) = 0.0;

            let colors: *mut u8 = libc::malloc(mem::size_of::<u8>() * 4 * vertex_count as usize) as *mut u8;
            for i in 0..3 as usize {
                *colors.wrapping_add(i * 4 + 0) = 0;
                *colors.wrapping_add(i * 4 + 1) = 128;
                *colors.wrapping_add(i * 4 + 2) = 255;
                *colors.wrapping_add(i * 4 + 3) = 255;
            }

            let indices: *mut u16 = libc::malloc(mem::size_of::<u16>() * 3) as *mut u16;

            *indices.add(0) = 0;
            *indices.add(1) = 1;
            *indices.add(2) = 2;
            let vbo: *mut u32 = libc::malloc(mem::size_of::<u32>() * 7) as *mut u32;
            for i in 0..7 {
                *vbo.wrapping_add(i) = 0;
            }
            let mesh = ffi::Mesh{
                vertexCount: vertex_count,
                triangleCount: triangle_count,
                vertices,
                texcoords,
                texcoords2: texcoords,
                normals,
                tangents: 0 as *mut f32,
                colors: 0 as *mut u8,
                indices,
                animVertices: 0 as *mut f32,
                animNormals: 0 as *mut f32,
                boneIds: 0 as *mut u8,
                boneWeights: 0 as *mut f32,
                vaoId: 0,
                vboId: vbo,
            };

            self.model = rl.load_model_from_mesh(thread, WeakMesh::from_raw(mesh)).expect("Error")

            /*
            let mesh = ffi::GenMeshCube(1.0, 1.0, 1.0);
            *mesh.vertices = 10.0;
            *mesh.normals.wrapping_add(0) = 0.07054;
            *mesh.normals.wrapping_add(1) = 0.70535;
            *mesh.normals.wrapping_add(2) = 0.70535;

            self.mesh = mesh;
            self.model = rl.load_model_from_mesh(thread, WeakMesh::from_raw(mesh)).expect("Error loading mesh");
            */
        }

    }
    
    
    fn add_voxel(&mut self, voxel: Voxel) {
        let x = (voxel.x - self.x).abs() as usize;
        let y = voxel.y as usize;
        self.voxels[y][x].push(voxel);
        self.voxels[y][x].sort_by(|a, b| a.compare_by_z(b));
    }
}

struct World {
    chunks: Vec<Chunk>,
    noise: worldgen::noise::perlin::PerlinNoise,
    seed: u64,
}
impl World {
    fn new() -> Self {
        let noise = PerlinNoise::new();
        World {
            chunks: Vec::new() as Vec<Chunk>,
            noise,
            seed: 69420,
        }
    }

    fn generate_chunk(&mut self, rl: &mut RaylibHandle, chunk_x: i64, chunk_z: i64, thread: &RaylibThread) {
        let chunk = Chunk::generate(rl, chunk_x, chunk_z, &self.noise, self.seed, thread);
        self.chunks.push(chunk);
    }
}

fn main() {
    // set up window
    let (mut rl, thread) = raylib::init()
        // .fullscreen()
        .vsync()
        .size(640, 480)
        .title("Spellcoder")
        .build();
    
    // rl.set_target_fps(60);
    rl.disable_cursor();
    // set up player
    let mut player = Player::new(Vector3::zero());
    let mut world = World::new();
    for x in 0..4 {
        for z in 0..4 {
            world.generate_chunk(&mut rl, x, z, &thread);
        }
    }
    // println!("{:?}", world.chunks[0].voxels);
    // mainloop
    let mut vel = Vector3::zero();
    println!("MAINLOOP STARTING");
    while !rl.window_should_close() {
        let delta = rl.get_frame_time();
        let _time = rl.get_time() as f32;
        // process input
        let mdelta = rl.get_mouse_delta(); // get mouse input
        let cam_angle = player.get_look_direction_vec2(); // get current yaw & pitch

        if (mdelta != Vector2 { x: 0.0, y: 0.0 }) {
            let new_cam_angle = Vector2 {
                x: cam_angle.x - mdelta.x / 180.0 * PI,
                y: cam_angle.y - (mdelta.y / 180.0 * PI),
            }; // construct new yaw & pitch vector
            player.set_look_direction_vec2(new_cam_angle);
        }
        let mut inputs = Vector2::zero();
        if rl.is_key_down(KeyboardKey::KEY_W) {
            inputs.x += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            inputs.x -= 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            inputs.y += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            inputs.y -= 1.0;
        }

        let cam_vec2 = Vector2 {
            x: player.get_look_direction().x,
            y: player.get_look_direction().z,
        }
        .normalized();
        inputs.normalize();
        inputs = Vector2::new(
            inputs.x * cam_vec2.x - inputs.y * cam_vec2.y,
            inputs.y * cam_vec2.x + inputs.x * cam_vec2.y,
        )
        .normalized()
            * SPEED;
        vel.x = inputs.x;
        vel.z = inputs.y;
        if player.position.y > 0.0 {
            vel.y -= 9.81 * 4.20 * delta;
        } else {
            vel.y = -player.position.y;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            vel.y += 32.0;
        }

        player.move_camera(vel * delta);
        if player.position.y < 0.0 {
            player.move_camera(Vector3 {
                x: 0.0,
                y: -player.position.y,
                z: 0.0,
            });
        }
        // set up drawing
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        // use d for 2d drawing here (background)

        let mut d3d = d.begin_mode3D(player.camera);

        // use d3d for 3d drawing here
        d3d.draw_grid(32, 1.0);
        /*unsafe {
            let mut voxels: [[[CVoxel; 16]; 16]; 65535] = [[[CVoxel {x: 0, y: 0, z: 0, color: Color::BLACK, visible_faces: [6; 6]} ; 16]; 16]; u16::MAX as usize];
            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..u16::MAX as usize {
                        voxels[x][y][z] = world.chunks[0].voxels[x][y][z].into();
                    }
                }
            }
            gen_chunk_mesh(voxels);
        }*/
        d3d.draw_world(&world);
        // d3d.draw_model(model, Vector3{x: 0.0, y: 0.0, z: 0.0}, 1.0, Color::WHITE);
        d3d.draw_bounding_box(
            BoundingBox {
                min: player.position,
                max: player.position + player.size,
            },
            Color::LIME,
        );
        drop(d3d);
        // use d for 2d drawing here (overlay)

        d.draw_fps(10, 10);
    }
}
