use std::f32::consts::PI;

use raylib::prelude::*;

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
                Vector3 { x: position.x + 8.0, y: position.y + 24.0, z: position.z + 8.0 },
                Vector3 { x: position.x + 9.0, y: position.y + 24.0, z: position.z + 8.0 },
                Vector3::up(),
                70.0
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
    y: i64,
    z: i64,
    color: ffi::Color,
}

trait VoxelDraw {
    fn draw_voxel(&mut self, voxel: Voxel);
}
impl VoxelDraw for RaylibMode3D<'_, RaylibDrawHandle<'_>> {
    fn draw_voxel(&mut self, voxel: Voxel) {
        // does what it says on the tin
        self.draw_cube(
            Vector3 {
                x: voxel.x as f32 + 0.5,
                y: voxel.y as f32 + 0.5,
                z: voxel.z as f32 + 0.5,
            },
            1.0,
            1.0,
            1.0,
            voxel.color,
        );
    }
}


fn main() {
    // set up window
    let (mut rl, thread) = raylib::init().size(640, 480).title("Spellcoder").build();
    rl.set_target_fps(60);
    rl.disable_cursor();
    // set up player
    let mut player = Player::new(Vector3::zero());
    player.set_look_direction_vec2(Vector2 {
        x: -3.0 * PI / 4.0,
        y: -0.5,
    });
    let mut i = 0;
    let mut j = 0;
    // mainloop
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

        let mut movement = Vector2::zero();
        if rl.is_key_down(KeyboardKey::KEY_W) {
            movement.x += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            movement.x -= 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            movement.y += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            movement.y -= 1.0;
        }

        if movement != Vector2::zero() {
            let cam_vec2 = Vector2 {
                x: player.get_look_direction().x,
                y: player.get_look_direction().z,
            }
            .normalized();
            movement.normalize();
            movement = Vector2::new(
                movement.x * cam_vec2.x - movement.y * cam_vec2.y,
                movement.y * cam_vec2.x + movement.x * cam_vec2.y,
            )
            .normalized()
                * SPEED
                * delta;
            player.move_camera(Vector3 {
                x: movement.x,
                y: 0.0,
                z: movement.y,
            });
        }

        // set up drawing
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // use d for 2d drawing here (background)

        let mut d3d = d.begin_mode3D(player.camera);

        // use d3d for 3d drawing here
        d3d.draw_grid(32, 1.0);
        d3d.draw_voxel(Voxel {
            x: 0,
            y: 0,
            z: 0,
            color: Color::from_hex("0080FF").unwrap().into(),
        });
        d3d.draw_voxel(Voxel {
            x: 2,
            y: 0,
            z: 0,
            color: Color::from_hex("FF8000").unwrap().into(),
        });

        d3d.draw_bounding_box(BoundingBox {min: player.position, max: player.position + player.size}, Color::WHITE);
        drop(d3d);
        // use d for 2d drawing here (overlay)

        d.draw_fps(10, 10);
        i += 1;
        if i % 2 == 0 {
            j += 1;
        }
    }
}
