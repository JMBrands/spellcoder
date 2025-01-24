use ::core::time;
use std::fmt::{self, format, Debug};
use ffi::{Color};
use raylib::prelude::*;
use worldgen::noise::{perlin::PerlinNoise, NoiseProvider};

const SPEED: f32 = 32.0;
const SCALE: i32 = 4;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
enum PixelMaterial {
    AIR,
    BLOCK
}

struct Player {
    position: Vector2,
    size: Vector2,
    camera: Camera2D,
}

#[derive(Clone, Copy)]
struct Pixel {
    x: u8, // first nibble for x, second nibble for z
    y: u8,
    material: PixelMaterial,
    color: ffi::Color,
}

struct Chunk {
    pixels: Vec<Vec<Pixel>>,
    x: i64,
    y: i64
}

struct World {
    chunks: Vec<Chunk>,
    noise: worldgen::noise::perlin::PerlinNoise,
    seed: u64,
}

trait WorldDraw {
    fn draw_chunk(&mut self, chunk: &Chunk);
    fn draw_world(&mut self, world: &World);
    fn draw_player(&mut self, player: &Player);
}

impl Player {
    fn new(position: Vector2) -> Self {
        let player = Player {
            position,
            size: Vector2 {
                x: 8.0,
                y: 16.0
            }, 
            camera: Camera2D {
                offset: position,
                target: position,
                rotation: 0.0,
                zoom: 1.0
            }
        };
        // player.set_look_direction_vec2(Vector2 {
        //     x: 0.0,
        //     y: -PI / 2.0,
        // });
        player
    }
    // move camera without changing yaw & pitch
    fn move_self(&mut self, delta: Vector2) {
        self.position += delta;
        self.camera.offset += delta;
        self.camera.target += delta;
    }
}

impl Debug for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pixel")
            .field("color", &self.color)
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl Pixel {
    fn compare_by_y(&self, other: &Self) -> std::cmp::Ordering {
        self.y.cmp(&other.y)
    }
}

impl WorldDraw for RaylibMode2D<'_, RaylibDrawHandle<'_>> {
    fn draw_chunk(&mut self, chunk: &Chunk) {
        for row in &chunk.pixels {
            for vox in row {
                self.draw_rectangle((vox.x as i32 + chunk.x as i32) * SCALE, (vox.y as i32 + chunk.y as i32) * SCALE, SCALE, SCALE, vox.color);
            }
        }
    }

    fn draw_player(&mut self, player: &Player) {
        self.draw_rectangle(player.position.x as i32 * SCALE, player.position.y as i32 * SCALE, player.size.x as i32 * SCALE, player.size.y as i32 * SCALE, Color {r: 255, g: 255, b: 255, a: 255});
    }

    fn draw_world(&mut self, world: &World) {
        for chunk in &world.chunks {
            self.draw_chunk(chunk);
        }
    }
}

impl Chunk {
    fn new(rl: &mut RaylibHandle, x: i64, y: i64, thread: &RaylibThread) -> Chunk {
        let mut pixels = Vec::with_capacity(16) as Vec<Vec<Pixel>>;
        for x in 0..16 as usize {
            pixels.push(Vec::with_capacity(16) as Vec<Pixel>);
        }
        let chunk = Chunk {
            pixels,
            x,
            y,
        };
        // for x in 0..16 as u8 {
        //     for y in 0..=65535 as u16 {
        //         for z in 0..16 as u8 {
        //             chunk.add_voxel(Voxel{material: VoxelMaterial::AIR, color: prelude::Color::WHITE.into(), visible_faces: [true; 6]}, x, y, z);
        //         }
        //     }
        // }
        chunk
    }

    fn generate(
        rl: &mut RaylibHandle,
        chunk_x: i64,
        chunk_y: i64,
        noise: &PerlinNoise,
        seed: u64,
        thread: &RaylibThread,
    ) -> Self {
        let mut chunk = Chunk::new(rl, chunk_x * 16, chunk_y * 16, thread);
        for x in 0..16 {
            for y in 0..16 {
                chunk.add_pixel(
                    Pixel {
                        color: Color {
                            r: (x * 16) as u8,
                            g: 255,
                            b: (y * 16) as u8,
                            a: 255,
                        }
                        .into(),
                        material: PixelMaterial::BLOCK,
                        x: x as u8,
                        y: y as u8
                    }
                );
                // println!("{}", noise.generate((chunk_x * 16 + x) as f64 / 32.0, (chunk_z * 16 + z) as f64 / 32.0, seed));
            }
        }
        
        chunk
    }
    
    fn add_pixel(&mut self, pixel: Pixel) {
        let x = pixel.x as usize;
        let y = pixel.y as usize;
        self.pixels[x].push(pixel);
        self.pixels[x].sort_by(|a, b| a.compare_by_y(&b));
    }

    fn get_pixel(&self, x: usize, y: usize) -> Result<&Pixel, usize> {
        match self.pixels[x].binary_search_by(|a| (a.y).cmp(&(y as u8))) {
            Ok(i) => Ok(&self.pixels[x][i]),
            Err(i) => Err(i)
        }
    }
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
        self.chunks.push(Chunk::generate(rl, chunk_x, chunk_z, &self.noise, self.seed, thread));
        // self.chunks.push(Chunk::new(rl, chunk_x, chunk_z, thread));
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
    // rl.disable_cursor();
    // set up player
    let mut player = Player::new(Vector2::zero());
    let mut world = World::new();
    for x in 0..4 {
        for z in 0..4 {
            world.generate_chunk(&mut rl, x, z, &thread);
        }
    }
    // println!("{:?}", world.chunks[0].voxels);
    // mainloop
    let mut vel = Vector2::zero();
    println!("MAINLOOP STARTING");
    while !rl.window_should_close() {
        let delta = rl.get_frame_time();
        let _time = rl.get_time() as f32;
        // process input

        let mut inputs = Vector2::zero();
        if rl.is_key_down(KeyboardKey::KEY_W) {
            inputs.y -= 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            inputs.y += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            inputs.x += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            inputs.x -= 1.0;
        }
        
        vel.x = inputs.x;
        if player.position.y < (rl.get_screen_height() as f32 / SCALE as f32 - player.size.y) {
            vel.y += 9.81 * delta;
        } else {
            vel.y = 0.0;
            player.move_self(Vector2 { x: 0.0, y: rl.get_screen_height() as f32 / SCALE as f32 - player.position.y - player.size.y });
        }

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) || inputs.y < 0.0 {
            vel.y -= 3.20;
        }

        player.move_self(vel);
        // set up drawing
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(prelude::Color::BLACK);
        // use d for 2d drawing here (background)
        let mut d2d = d.begin_mode2D(player.camera);
        /*
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
            prelude::Color::LIME,
        );
        drop(d3d);
        */
        // use d for 2d drawing here (overlay)
        d2d.draw_world(&world);
        d2d.draw_player(&player);
        drop(d2d);
        d.draw_fps(10, 10);
        d.draw_text(&(format!("{}, {}", player.position.x, player.position.y).as_str()), 10, 30, 20, Color {r:0, g: 179, b: 0, a: 255});
    }
}
