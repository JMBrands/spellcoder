use ffi::Color;
use glob::glob;
use interpolation::{self, Ease, EaseFunction};
use jzon::{object::Object, parse, JsonValue};
use rand::prelude::*;
use raylib::{ease::quad_in, ffi::Font, prelude::*};
use std::{
    arch::x86_64, fmt::{self, Debug}, fs::read_to_string
};
use worldgen::noise::{perlin::PerlinNoise, NoiseProvider};

const SPEED: f32 = 2.0;
const SCALE: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
enum PixelMaterial {
    AIR,
    BLOCK,
}

enum SpellComponent {
    SetPixel(i64, i64, PixelMaterial, Color, Events),
    Damage(*mut Player, f32),
    Nothing
}

struct Spell {
    name: String,
    components: Vec<SpellComponent>,
    cost: f32,
}

struct Events {
    on_touch: Vec<SpellComponent>,
}

struct Player {
    position: Vector2,
    size: Vector2,
    camera: Camera2D,
    mp: f32,
    hp: f32,
    sp: f32,
    max_mp: f32,
    max_hp: f32,
    max_sp: f32,
    display_mp: f32,
    display_hp: f32,
    display_sp: f32,
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
    y: i64,
}

struct World {
    chunks: Vec<Vec<Chunk>>,
    noise: worldgen::noise::perlin::PerlinNoise,
    seed: u64,
    rng: ThreadRng,
    modified: bool,
}

trait WorldDraw {
    fn draw_chunk(&mut self, chunk: &Chunk);
    fn draw_world(&mut self, world: &mut World, camera: &Camera2D, screendims: Vector2);
    fn draw_player(&mut self, player: &Player);
    fn get_visible_chunks(
        &mut self,
        camera: &Camera2D,
        screendims: Vector2,
    ) -> [::core::ops::Range<i64>; 2];
}
trait HUDDraw {
    fn draw_hud(&mut self, world: &World, player: &Player, active_spell: &Spell);
}

impl Player {
    fn new(position: Vector2) -> Self {
        let player = Player {
            position,
            size: Vector2 { x: 8.0, y: 16.0 },
            camera: Camera2D {
                offset: Vector2 { x: 0.0, y: 0.0 },
                target: position,
                rotation: 0.0,
                zoom: 1.0,
            },
            mp: 100.0,
            hp: 100.0,
            sp: 100.0,
            max_mp: 100.0,
            max_hp: 100.0,
            max_sp: 100.0,
            display_mp: 100.0,
            display_hp: 100.0,
            display_sp: 100.0,
        };
        // player.set_look_direction_vec2(Vector2 {
        //     x: 0.0,
        //     y: -PI / 2.0,
        // });
        player
    }
    // move camera without changing yaw & pitch
    fn move_self(&mut self, delta: Vector2) {
        let newpos = self.position + delta;
        self.position = newpos;
        self.camera.target -= (self.camera.target / SCALE as f32 - self.position) / 3.0;
        // self.camera.offset += delta;
    }

    fn set_hp(&mut self, hp: f32) {
        self.hp = hp.clamp(0.0, self.max_hp);
    }

    fn activate_spell(&mut self, spell: &Spell, world: &mut World) -> () {
        if self.mp < spell.cost {
            ()
        } else {
            self.mp -= spell.cost;
            for component in &spell.components {
                match component {
                    SpellComponent::Damage(target, amount) => unsafe {(**target).set_hp((**target).hp - *amount)},
                    SpellComponent::SetPixel(x_rel, y_rel, mat, color, events) => {
                        let x = self.position.x as i64 + x_rel;
                        let y = self.position.y as i64 + y_rel;
                        world.set_pixel(x, y, Pixel { x: match (x % 16) as u8 {
                            a if a < 16 => a,
                            b if b > 240 => b - 240,
                            e => panic!("no idea how this could happen, x % 16 is {}", e)
                        }, y: match (y % 16) as u8 {
                            a if a < 16 => a,
                            b if b > 240 => b - 240,
                            e => panic!("no idea how this could happen, y % 16 is {}", e)
                        }, material: *mat, color: *color });
                    },
                    SpellComponent::Nothing => ()
                }
            }
        }
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
        f.debug_struct("Chunk")
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
                self.draw_rectangle(
                    (vox.x as i32 + chunk.x as i32) * SCALE,
                    (vox.y as i32 + chunk.y as i32) * SCALE,
                    SCALE,
                    SCALE,
                    vox.color,
                );
            }
        }
    }

    fn draw_player(&mut self, player: &Player) {
        self.draw_rectangle(
            player.position.x as i32 * SCALE,
            player.position.y as i32 * SCALE,
            player.size.x as i32 * SCALE,
            player.size.y as i32 * SCALE,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        );
    }

    fn draw_world(&mut self, world: &mut World, camera: &Camera2D, screendims: Vector2) {
        let visible = self.get_visible_chunks(camera, screendims);
        for y in visible[1].clone() {
            for x in visible[0].clone() {
                self.draw_chunk(world.get_chunk(x, y));
            }
        }
    }

    fn get_visible_chunks(
        &mut self,
        camera: &Camera2D,
        screendims: Vector2,
    ) -> [::core::ops::Range<i64>; 2] {
        let min = ((camera.target + camera.offset) - Vector2 { x: 64.0, y: 128.0 } - screendims)
            / 16.0
            / SCALE as f32;
        let max =
            ((camera.target + camera.offset) + Vector2 { x: 64.0, y: 128.0 }) / 16.0 / SCALE as f32;
        [min.x as i64..max.x as i64, min.y as i64..max.y as i64]
    }
}

impl HUDDraw for RaylibDrawHandle<'_> {
    fn draw_hud(&mut self, world: &World, player: &Player, active_spell: &Spell) {
        let hpcol = Color {
            r: 255 - (player.display_hp / player.max_hp * 255.0) as u8,
            g: (player.display_hp / player.max_hp * 255.0) as u8,
            b: 0,
            a: 255,
        };

        let mpcol = Color {
            r: 0,
            g: 0u8.max((player.display_mp / player.max_mp * 255.0 - 127.0) as u8),
            b: (player.display_mp / player.max_mp * 191.0) as u8 + 64,
            a: 255,
        };

        let spcol = Color {
            r: (player.display_sp / player.max_sp * 191.0) as u8 + 64,
            g: (player.display_sp / player.max_sp * 191.0) as u8 + 64,
            b: 0,
            a: 255,
        };

        let max_x = self.get_screen_width() as f32;
        let max_y = self.get_screen_height() as f32;
        self.draw_rectangle(
            (max_x * 0.02) as i32,
            (max_y * 0.84) as i32,
            (max_x * 0.22) as i32,
            (max_y * 0.145) as i32,
            Color {
                r: 32,
                g: 32,
                b: 25,
                a: 255,
            },
        );
        self.draw_rectangle(
            (max_x * 0.03) as i32,
            (max_y * 0.86) as i32,
            (max_x * 0.2 * player.display_hp / player.max_hp) as i32,
            (max_y * 0.025) as i32,
            hpcol,
        );
        self.draw_rectangle(
            (max_x * 0.03) as i32,
            (max_y * 0.90) as i32,
            (max_x * 0.2 * player.display_mp / player.max_mp) as i32,
            (max_y * 0.025) as i32,
            mpcol,
        );
        self.draw_rectangle(
            (max_x * 0.03) as i32,
            (max_y * 0.94) as i32,
            (max_x * 0.2 * player.display_sp / player.max_sp) as i32,
            (max_y * 0.025) as i32,
            spcol,
        );

        self.draw_text(&active_spell.name.as_str(), 15, (max_y / 2.0) as i32, 35, prelude::Color::BLACK);
    }
}

impl Chunk {
    fn new(x: i64, y: i64) -> Chunk {
        let mut pixels = Vec::with_capacity(16) as Vec<Vec<Pixel>>;
        for _x in 0..16 as usize {
            pixels.push(Vec::with_capacity(16) as Vec<Pixel>);
        }
        let chunk = Chunk { pixels, x, y };
        // for x in 0..16 as u8 {
        //     for y in 0..=65535 as u16 {
        //         for z in 0..16 as u8 {
        //             chunk.add_voxel(Voxel{material: VoxelMaterial::AIR, color: prelude::Color::WHITE.into(), visible_faces: [true; 6]}, x, y, z);
        //         }
        //     }
        // }
        chunk
    }

    fn generate(chunk_x: i64, chunk_y: i64, noise: &PerlinNoise, seed: u64) -> Self {
        let mut chunk = Chunk::new(chunk_x * 16, chunk_y * 16);
        for x in 0..16 {
            for y in 0..16 {
                let val = noise.generate(
                    (x + chunk_x * 16) as f64 / 320.0,
                    (y + chunk_y * 16) as f64 / 128.0,
                    seed,
                );
                match val {
                    val if val > 80.0 / 256.0 => {
                        let gval = (noise.generate(
                            (x + chunk_x * 16) as f64 / 32.0,
                            (y + chunk_y * 16) as f64 / 32.0,
                            seed,
                        ) * 16.0
                            + 128.0) as u8;
                        chunk.add_pixel(Pixel {
                            color: Color {
                                r: gval,
                                g: gval,
                                b: gval,
                                a: 255,
                            }
                            .into(),
                            material: PixelMaterial::BLOCK,
                            x: x as u8,
                            y: y as u8,
                        });
                    }
                    val if val > 64.0 / 256.0 => {
                        let gval = (noise.generate(
                            (x + chunk_x * 16) as f64 / 32.0,
                            (y + chunk_y * 16) as f64 / 32.0,
                            seed,
                        ) * 32.0
                            + 128.0) as u8;
                        chunk.add_pixel(Pixel {
                            color: Color {
                                r: 0,
                                g: gval,
                                b: 0,
                                a: 255,
                            }
                            .into(),
                            material: PixelMaterial::BLOCK,
                            x: x as u8,
                            y: y as u8,
                        });
                    }
                    val if val < 64.0 / 256.0 => {
                        let gval = (noise.generate(
                            (x + chunk_x * 16) as f64 / 96.0,
                            (y + chunk_y * 16) as f64 / 32.0,
                            seed,
                        ));
                        chunk.add_pixel(Pixel {
                            color: Color {
                                r: 255,
                                g: 255,
                                b: 255,
                                a: ((gval * 200.0 * (64.0 / 256.0 - val).cubic_out().cubic_out())
                                    .clamp(0.0, 255.0) as u8),
                            }
                            .into(),
                            material: PixelMaterial::AIR,
                            x: x as u8,
                            y: y as u8,
                        });
                    }
                    _ => {
                        chunk.add_pixel(Pixel {
                            color: Color {
                                g: 0,
                                b: 0,
                                r: 0,
                                a: 0,
                            }
                            .into(),
                            x: x as u8,
                            y: y as u8,
                            material: PixelMaterial::AIR,
                        });
                    }
                }

                // println!("{}", noise.generate((chunk_x * 16 + x) as f64 / 32.0, (chunk_z * 16 + z) as f64 / 32.0, seed));
            }
        }

        chunk
    }

    fn add_pixel(&mut self, pixel: Pixel) {
        let x = pixel.x as usize;
        // let y = pixel.y as usize;
        self.pixels[x].push(pixel);
        self.pixels[x].sort_by(|a, B| a.compare_by_y(&B));
    }

    fn get_pixel(&self, x: usize, y: usize) -> Result<&Pixel, usize> {
        match self.pixels[x].binary_search_by(|a| (a.y).cmp(&(y as u8))) {
            Ok(i) => Ok(&self.pixels[x][i]),
            Err(i) => Err(i),
        }
    }

    fn set_pixel(&mut self, pixel: Pixel) {
        println!("{}, {}", pixel.x, pixel.x as usize);
        match self.pixels[pixel.x as usize].binary_search_by(|a| (a.y).cmp(&(pixel.y as u8))) {
            Ok(i) => self.pixels[pixel.x as usize][i] = pixel,
            Err(i) => self.add_pixel(pixel),
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
            modified: false,
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

    fn get_chunk(&mut self, x: i64, y: i64) -> &mut Chunk {
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
        &mut self.chunks[row][col]
    }

    fn get_pixel(&mut self, x: i64, y: i64) -> &Pixel {
        // 0b100000 >> 4 = 0b000010
        let chunk = self.get_chunk(x >> 4, y >> 4);
        match chunk.get_pixel((x as usize) % 16, (y as usize) % 16) {
            Ok(p) => p,
            Err(_) => panic!("pixel not found! (how?)"),
        }
    }

    fn set_pixel(&mut self, x: i64, y: i64, pixel: Pixel) {
        let chunk = self.get_chunk(x >> 4, y >> 4);
        println!("{}", pixel.x);
        chunk.set_pixel(pixel);
    }
}

fn parse_components<'a>(components: &mut Vec<SpellComponent>, json: &JsonValue, player: &mut Player) -> f32 {
    let mut cost = 0f32;
    for comp in json.as_array().unwrap() {
        components.push(match comp["type"].as_str().unwrap() {
            "setpixel" => {
                cost += 16.0;
                SpellComponent::SetPixel(
                    comp["position"]["x"].as_i64().unwrap(),
                    comp["position"]["y"].as_i64().unwrap(),
                    match comp["material"].as_str().unwrap() {
                        "air" => PixelMaterial::AIR,
                        "block" => PixelMaterial::BLOCK,
                        _ => PixelMaterial::AIR,
                    },
                    prelude::Color::from_hex(comp["color"].as_str().unwrap()).unwrap().into(),
                    Events { on_touch: 
                        if comp["events"].has_key("on_touch") {
                            let mut tch_comps = Vec::new() as Vec<SpellComponent>;
                            cost += parse_components(&mut tch_comps, &comp["events"]["on_touch"], player) * 1.5;
                            tch_comps
                        } else {
                            vec![SpellComponent::Nothing]
                        }
                    }
                )
            },
            "damage" => {
                cost += comp["amount"].as_f32().unwrap() * 8.0;
                SpellComponent::Damage(player, comp["amount"].as_f32().unwrap())
            },
            _ => SpellComponent::Nothing
        });
    }
    cost
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
        y: rl.get_screen_height() as f32 / 2.0,
    };
    let mut world = World::new();
    
    let mut spells: Vec<Spell> = Vec::new() as Vec<Spell>;
    let spellpaths = glob("./spells/*.json").unwrap();
    for spellpath in spellpaths {
        match spellpath {
            Err(e) => println!("{:#?}", e),
            Ok(s) => {
                let contents = read_to_string(s).unwrap();
                let sp = jzon::parse(&contents).unwrap();
                for s in sp.as_array().unwrap() {
                    // println!("{:#?}", s["components"][0]["position"]["x"]);
                    let mut components = Vec::new() as Vec<SpellComponent>;
                    let cost = parse_components(&mut components, &s["components"], &mut player);
                    spells.push(Spell {
                        name: String::from(s["name"].as_str().unwrap()),
                        components,
                        cost
                    });
                }
            }
        };
    }
    // for x in -16..16 {
    //     for y in -16..16 {
    //         world.generate_chunk(x, y,);
    //     }
    // }
    // println!("{:?}", world.chunks[0].voxels);
    // mainloop
    let mut vel = Vector2::zero();
    let mut active_index = 0usize;
    let mut active_spell = &spells[active_index];
    let mut jump_time = 0.0;
    // let mut screen_dim = Vector2 {x: }
    println!("MAINLOOP STARTING");
    let mut screendim = Vector2 {
        x: rl.get_screen_width() as f32,
        y: rl.get_screen_height() as f32,
    };
    let mut coyotetime = 0.1;
    while !rl.window_should_close() {
        if vel.y == 0.0 {
            coyotetime = 0.1;
        }
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

        if rl.is_key_down(KeyboardKey::KEY_P) {
            player.hp = player.max_hp.min(player.hp + 3.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_O) {
            player.hp = 0f32.max(player.hp - 3.0);
        }

        if rl.is_key_down(KeyboardKey::KEY_L) {
            player.mp = player.max_mp.min(player.mp + 3.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_K) {
            player.mp = 0f32.max(player.mp - 3.0);
        }

        if rl.is_key_down(KeyboardKey::KEY_M) {
            player.sp = player.max_sp.min(player.sp + 3.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_N) {
            player.sp = 0f32.max(player.sp - 3.0);
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            player.activate_spell(active_spell, &mut world);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if active_index == 0 {
                active_index = spells.len() - 1;
            } else {
                active_index -= 1;
            }
            active_spell = &spells[active_index];
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            if active_index == spells.len() - 1 {
                active_index = 0;
            } else {
                active_index += 1;
            }
            active_spell = &spells[active_index];
        }

        player.display_hp = lerp(player.display_hp, player.hp, 0.1);
        player.display_mp = lerp(player.display_mp, player.mp, 0.1);
        player.display_sp = lerp(player.display_sp, player.sp, 0.1);

        vel.x = inputs.x * SPEED;
        let mut newpos = player.position + delta;
        let mut emptycount = 0;
        for x in (newpos.x as i64)..(newpos.x as i64 + 8) {
            let bottompx = world.get_pixel(x, newpos.y as i64 + 16);
            if bottompx.material == PixelMaterial::AIR {
                emptycount += 1;
            } else {
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
        if emptycount == 8 {
            vel.y += 9.81 * delta;
        }

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

        if (rl.is_key_pressed(KeyboardKey::KEY_SPACE) || inputs.y < 0.0) && coyotetime > 0.0 && player.sp > 5.0 {
            vel.y -= 3.20;
            coyotetime = 0.0;
            player.sp -= 5.0;
            jump_time = 0.0;
        }

        player.move_self(vel);
        // set up drawing
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(prelude::Color::CYAN);
        // use d for 2d drawing background here
        let mut d2d = d.begin_mode2D(player.camera);
        // use d2d for 2d drawing game here
        d2d.draw_world(&mut world, &player.camera, screendim);
        d2d.draw_player(&player);
        drop(d2d);
        // use d for drawing hud here
        d.draw_fps(10, 10);
        d.draw_text(
            &(format!("{}, {}", player.position.x, player.position.y).as_str()),
            10,
            30,
            20,
            Color {
                r: 0,
                g: 179,
                b: 0,
                a: 255,
            },
        );
        d.draw_text(
            &(format!("{}, {}", vel.x, vel.y).as_str()),
            10,
            50,
            20,
            Color {
                r: 0,
                g: 179,
                b: 0,
                a: 255,
            },
        );
        d.draw_hud(&world, &player, &active_spell);
        // world.sort_chunks();
        if world.modified {
            world.sort_chunks();
        }
        if player.mp < player.max_mp {
            player.mp = (player.mp + 2.0 * delta).min(player.max_mp);
        }
        if player.sp < player.max_sp && jump_time > 2.0 {
            player.sp = (player.sp + 35.0 * delta).min(player.max_sp);
        }
        coyotetime = 0f32.max(coyotetime - delta);
        jump_time += delta;
    }
}

/*0010 0010 0100 0010 0010 0101 0010 1010
  1 
  1 
 1  
  1 
  1 
 1 1
  1 
1 1 
*/