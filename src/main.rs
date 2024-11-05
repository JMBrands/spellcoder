use std::{
    cmp::Ordering,
    f32::consts::PI,
    fmt::{self, Debug},
    i8,
    thread::panicking,
};

use raylib::prelude::*;
use worldgen::noise::{perlin::PerlinNoise, NoiseProvider};

const SPEED: f32 = 32.0;

struct Player {
    position: Vector3,
    camera: Camera3D,
    size: Vector3,
}
impl Player {
    fn new(position: Vector3) -> Self {
        Player {
            position,
            camera: Camera3D::perspective(
                Vector3 {
                    x: position.x + 8.0,
                    y: position.y + 24.0,
                    z: position.z + 8.0,
                },
                Vector3 {
                    x: position.x + 9.0,
                    y: position.y + 24.0,
                    z: position.z + 8.0,
                },
                Vector3::up(),
                70.0,
            ),
            size: Vector3 {
                x: 16.0,
                y: 32.0,
                z: 16.0,
            },
        }
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

struct Voxel {
    x: i64,
    y: u16,
    z: i64,
    color: ffi::Color,
    visible_faces: Vec<i8>, // Vector with face indices for every face that's visible, the other faces will not be drawn.
                            // 0 = down 1 = up 2 = north 3 = south 5 = east 6 = west
}
impl Debug for Voxel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Voxel")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("z", &self.z)
            .field("color", &self.color)
            .field("visible_faces", &self.visible_faces)
            .finish()
    }
}
impl Voxel {
    fn compare_by_z(&self, b: &Voxel) -> Ordering {
        self.z.cmp(&b.z)
    }
}

trait VoxelDraw {
    fn draw_voxel(&mut self, voxel: &Voxel);
    fn draw_chunk(&mut self, chunk: &Chunk);
}
impl VoxelDraw for RaylibMode3D<'_, RaylibDrawHandle<'_>> {
    fn draw_voxel(&mut self, voxel: &Voxel) {
        let position = Vector3::new(voxel.x as f32, voxel.y as f32, voxel.z as f32);
        for face in &voxel.visible_faces {
            match face {
                0 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::forward() + Vector3::right(),
                        position + Vector3::forward(),
                        position + Vector3::right(),
                        position,
                    ],
                    voxel.color,
                ),
                1 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::up(),
                        position + Vector3::forward() + Vector3::up(),
                        position + Vector3::right() + Vector3::up(),
                        position + Vector3::forward() + Vector3::right() + Vector3::up(),
                    ],
                    voxel.color,
                ),
                2 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::forward() + Vector3::up(),
                        position + Vector3::up(),
                        position + Vector3::forward(),
                        position,
                    ],
                    voxel.color,
                ),
                3 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::right(),
                        position + Vector3::up() + Vector3::right(),
                        position + Vector3::forward() + Vector3::right(),
                        position + Vector3::forward() + Vector3::up() + Vector3::right(),
                    ],
                    voxel.color,
                ),
                4 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::up() + Vector3::right(),
                        position + Vector3::right(),
                        position + Vector3::up(),
                        position,
                    ],
                    voxel.color,
                ),
                5 => self.draw_triangle_strip3D(
                    &[
                        position + Vector3::forward(),
                        position + Vector3::right() + Vector3::forward(),
                        position + Vector3::up() + Vector3::forward(),
                        position + Vector3::up() + Vector3::right() + Vector3::forward(),
                    ],
                    voxel.color,
                ),
                i8::MIN..=-1_i8 | 6_i8..=i8::MAX => (),
            }
        }
    }

    fn draw_chunk(&mut self, chunk: &Chunk) {
        for layer in &chunk.voxels {
            for row in layer {
                for voxel in row {
                    self.draw_voxel(voxel);
                }
            }
        }
    }
}

struct Chunk {
    voxels: Vec<Vec<Vec<Voxel>>>,
    x: i64,
    z: i64,
}
impl Chunk {
    fn new(x: i64, z: i64) -> Chunk {
        let mut voxels: Vec<Vec<Vec<Voxel>>> = Vec::new();
        for _y in i8::MIN..i8::MAX {
            let mut layer: Vec<Vec<Voxel>> = Vec::new();
            for _x in 0..16 {
                layer.push(Vec::new() as Vec<Voxel>);
            }
            voxels.push(layer);
        }
        Chunk { voxels, x, z }
    }

    fn generate(chunk_x: i64, chunk_z: i64, noise: &PerlinNoise, seed: u64) -> Self {
        let mut chunk = Chunk::new(chunk_x * 16, chunk_z * 16);
        for x in 0..16 {
            for z in 0..16 {
                chunk.add_voxel(Voxel {
                    x: chunk_x * 16 + x,
                    y: (noise.generate((chunk_x + x) as f64 / 48.0, (chunk_z + z) as f64 / 48.0, seed)*10.0) as u16,
                    z: chunk_z * 16 + z,
                    color: Color {
                        r: (x * 16) as u8,
                        g: 255,
                        b: (z * 16) as u8,
                        a: 255
                    }.into(),
                    visible_faces: vec![1,2,3,4,5],
                });
            }
        }
        chunk
    }

    fn add_voxel(&mut self, voxel: Voxel) {
        let x = (voxel.x - self.x).abs() as usize;
        let y = voxel.y as usize;
        println!("{} -> {}", voxel.y, y);
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

    fn generate_chunk(&mut self, chunk_x: i64, chunk_z: i64) {
        let chunk = Chunk::generate(chunk_x, chunk_z, &self.noise, self.seed);
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
    world.generate_chunk(0, 0);
    world.chunks[0].add_voxel(Voxel {
        x: 2,
        y: 16,
        z: 2,
        color: Color::from_hex("0080FF").unwrap().into(),
        visible_faces: vec![0, 1, 2, 3, 4, 5],
    });
    world.chunks[0].add_voxel(Voxel {
        x: 2,
        y: 0,
        z: 0,
        color: Color::from_hex("FF8000").unwrap().into(),
        visible_faces: vec![0, 1, 2, 3, 4, 5],
    });
    println!("{:?}", world.chunks[0].voxels);
    // mainloop
    let mut vel = Vector3::zero();
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
        d3d.draw_chunk(&world.chunks[0]);
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
