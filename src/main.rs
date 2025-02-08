use ::core::time;
use std::{fmt::{self, format, Debug}, sync::Arc};
use ffi::{Color};
use libc::{PRIO_MIN, SECCOMP_RET_KILL_THREAD};
use raylib::prelude::*;
use rand::prelude::*;
use worldgen::noise::{perlin::PerlinNoise, NoiseProvider};

const SPEED: f32 = 32.0;
const SCALE: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
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
    chunks: Vec<Vec<Chunk>>,
    noise: worldgen::noise::perlin::PerlinNoise,
    seed: u64,
    rng: ThreadRng,
    modified: bool
}

trait WorldDraw {
    fn draw_chunk(&mut self, chunk: &Chunk);
    fn draw_world(&mut self, world: &mut World, camera: &Camera2D, screendims: Vector2);
    fn draw_player(&mut self, player: &Player);
    fn get_visible_chunks(&mut self, camera: &Camera2D, screendims: Vector2) -> [::core::ops::Range<i64>; 2];
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
                offset: Vector2 { x: 0.0, y: 0.0 },
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
    fn move_self(&mut self, delta: Vector2, world: &mut World) {
        let newpos = self.position + delta;
        self.position = newpos;
        self.camera.target = newpos * SCALE as f32;
        // self.camera.target -= (self.camera.target / SCALE as f32 - self.position) / 3.0;
        // self.camera.offset += delta;
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

impl Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk").field("x", &self.x).field("y", &self.y).finish()
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

    fn draw_world(&mut self, world: &mut World, camera: &Camera2D, screendims: Vector2) {
        let visible = self.get_visible_chunks(camera, screendims);
        for y in visible[1].clone() {
            for x in visible[0].clone() {
                self.draw_chunk(world.get_chunk(x, y));
            }
        }
    }
    
    fn get_visible_chunks(&mut self, camera: &Camera2D, screendims: Vector2) -> [::core::ops::Range<i64>; 2] {
        let min = ((camera.target + camera.offset) - Vector2{x: 64.0, y: 128.0} - screendims) / 16.0 / SCALE as f32;
        let max = ((camera.target + camera.offset) + Vector2{x: 64.0, y: 128.0}) / 16.0 / SCALE as f32;
        [
            min.x as i64..max.x as i64,
            min.y as i64..max.y as i64
        ]
    }
}

impl Chunk {
    fn new(x: i64, y: i64) -> Chunk {
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
        chunk_x: i64,
        chunk_y: i64,
        noise: &PerlinNoise,
        seed: u64,
    ) -> Self {
        let mut chunk = Chunk::new(chunk_x * 16, chunk_y * 16);
        for x in 0..16 {
            for y in 0..16 {
                let val = noise.generate((x + chunk_x * 16) as f64 / 320.0, (y + chunk_y * 16) as f64 / 128.0, seed);
                if val > 64.0/256.0 {
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
                } else {
                    chunk.add_pixel(
                        Pixel {
                            color: Color {
                                r: (val * 255.0) as u8,
                                g: (val * 255.0) as u8,
                                b: (val * 255.0) as u8,
                                a: 255
                            }.into(),
                            x: x as u8,
                            y: y as u8,
                            material: PixelMaterial::AIR
                        }
                    );
                }
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
        let mut rng = rand::rng();
        World {
            chunks: Vec::new() as Vec<Vec<Chunk>>,
            noise,
            seed: rng.random::<u64>(),
            rng,
            modified: false
        }
    }

    fn generate_chunk(&mut self, chunk_x: i64, chunk_y: i64) {
        let row = match self.chunks.binary_search_by(|row| row[0].y.cmp(&chunk_y)) {
            Ok(row) => row,
            Err(_) => {
                self.chunks.push(Vec::new() as Vec<Chunk>);
                self.chunks.len() - 1
            }
        };
        self.chunks[row].push(Chunk::generate(chunk_x, chunk_y, &self.noise, self.seed));
        self.modified = true;
        // self.chunks.push(Chunk::new(rl, chunk_x, chunk_z, thread));
    }
    
    fn sort_chunks(&mut self) {
        self.chunks.sort_by(|ra, rb| ra[0].y.cmp(&rb[0].y));
        for i in 0..self.chunks.len() {
            self.chunks[i].sort_by(|ca, cb| ca.x.cmp(&cb.x));
        }
        self.modified = false;
    }

    fn get_chunk(&mut self, x: i64, y: i64) -> &Chunk {
        let row = match self.chunks.binary_search_by(|r| r[0].y.cmp(&(y * 16))) {
            Ok(r) => r,
            Err(_) => {
                self.chunks.push(Vec::new() as Vec<Chunk>);
                self.chunks.len() - 1
            }
        };
        let col = match self.chunks[row].binary_search_by(|c| c.x.cmp(&(x * 16))) {
            Ok(col) => col,
            Err(_) => {
                // println!("generating ({}, {})", x, y);
                self.chunks[row].push(Chunk::generate(x, y, &self.noise, self.seed));
                self.modified = true;
                self.chunks[row].len() - 1
            }
        };
        &self.chunks[row][col]
    }

    fn get_pixel(&mut self, x: i64, y: i64) -> &Pixel {
        // 0b100000 >> 4 = 0b000010
        let chunk = self.get_chunk(x >> 4, y >> 4);
        match chunk.get_pixel((x as usize) % 16 , (y as usize) % 16) {
            Ok(P) => P,
            Err(_) => panic!("pixel not found! (how?)")
        }
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
    // rl.set_target_fps(2);
    // rl.disable_cursor();
    // set up player
    let mut player = Player::new(Vector2::zero());
    player.camera.offset = Vector2 {
        x: rl.get_screen_width() as f32 / 2.0,
        y: rl.get_screen_height() as f32 / 2.0
    };
    let mut world = World::new();
    // for x in -16..16 {
    //     for y in -16..16 {
    //         world.generate_chunk(x, y,);
    //     }
    // }
    // println!("{:?}", world.chunks[0].voxels);
    // mainloop
    let mut vel = Vector2::zero();
    // let mut screen_dim = Vector2 {x: }
    println!("MAINLOOP STARTING");
    let mut screendim = Vector2 {x: rl.get_screen_width() as f32, y: rl.get_screen_height() as f32};
    while !rl.window_should_close() {
        let width = rl.get_screen_width() as f32;
        if screendim.x != width {
            screendim.x = width;
            player.camera.offset.x = screendim.x / 2.0;
        }
        let height = rl.get_screen_height() as f32;
        if screendim.y != height {
            screendim.y = height;
            player.camera.offset.y = screendim.y / 2.0;
        }
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
        let mut newpos = player.position + delta;
        let mut emptycount = 0;
        for x in (newpos.x as i64)..(newpos.x as i64 + 8) {
            let bottompx = world.get_pixel(x, newpos.y as i64 + 16);
            if bottompx.material == PixelMaterial::AIR {emptycount += 1;}
            else {
                let mut toppx = bottompx;
                let mut y = newpos.y as i64 + 16;
                while toppx.material != PixelMaterial::AIR {
                    toppx = world.get_pixel(x, y);
                    y -= 1;
                }
                vel.y = 0.0;
                // println!("{:#?}, {}", toppx, y);
                if newpos.y > y as f32 - 14.0 {
                    newpos.y = y as f32 - 14.0;
                }
                player.position.y = newpos.y;
            }
        }
        if emptycount == 8 {vel.y += 9.81 * delta;}

        for x in (newpos.x as i64)..(newpos.x as i64 + 8) {
            let bottompx = world.get_pixel(x, newpos.y as i64);
            if bottompx.material != PixelMaterial::AIR {
                let mut toppx = bottompx;
                let mut y = newpos.y as i64;
                while toppx.material != PixelMaterial::AIR {
                    toppx = world.get_pixel(x, y);
                    y += 1;
                }
                vel.y = 0.0;
                // println!("{:#?}, {}", toppx, y);
                if newpos.y < y as f32 + 2.0 {
                    newpos.y = y as f32 + 2.0;
                }
                player.position.y = newpos.y;
            }
        }     

        for y in (newpos.y as i64)..(newpos.y as i64 + 12) {
            let bottompx = world.get_pixel(newpos.x as i64, y);
            if bottompx.material != PixelMaterial::AIR {
                let mut toppx = bottompx;
                let mut x = newpos.x as i64;
                while toppx.material != PixelMaterial::AIR {
                    toppx = world.get_pixel(x, y);
                    x += 1;
                }
                vel.x = 0.0;
                // println!("{:#?}, {}", toppx, y);
                if newpos.x < x as f32 - 3.0 {
                    newpos.x = x as f32 - 3.0;
                }
                player.position.x = newpos.x;
            }
        }

        for y in (newpos.y as i64)..(newpos.y as i64 + 12) {
            let bottompx = world.get_pixel(newpos.x as i64 + 8, y);
            if bottompx.material != PixelMaterial::AIR {
                let mut toppx = bottompx;
                let mut x = newpos.x as i64 + 8;
                while toppx.material != PixelMaterial::AIR {
                    toppx = world.get_pixel(x, y);
                    x -= 1;
                }
                vel.x = 0.0;
                // println!("{:#?}, {}", toppx, y);
                if newpos.x > x as f32 + 5.0 {
                    newpos.x = x as f32 + 5.0;
                }
                player.position.x = newpos.x;
            }
        }

        if (rl.is_key_pressed(KeyboardKey::KEY_SPACE) || inputs.y < 0.0) && vel.y == 0.0 {
            vel.y -= 3.20;
        }

        player.move_self(vel, &mut world);
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
        d2d.draw_world(&mut world, &player.camera, screendim);
        d2d.draw_player(&player);
        drop(d2d);
        d.draw_fps(10, 10);
        d.draw_text(&(format!("{}, {}", player.position.x, player.position.y).as_str()), 10, 30, 20, Color {r:0, g: 179, b: 0, a: 255});
        d.draw_text(&(format!("{}, {}", vel.x, vel.y).as_str()), 10, 50, 20, Color {r:0, g: 179, b: 0, a: 255});
        // world.sort_chunks();
        if world.modified {
            world.sort_chunks();
        }
    }
}
