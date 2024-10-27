use std::f32::consts::PI;

use raylib::prelude::*;

struct Voxel {
    x: i64,
    y: i64,
    z: i64,
    color: ffi::Color
}

trait VoxelDraw {
    fn draw_voxel(&mut self, voxel: Voxel);
}
impl VoxelDraw for RaylibMode3D<'_, RaylibDrawHandle<'_>> {
    fn draw_voxel(&mut self, voxel: Voxel) { // does what it says on the tin
        self.draw_cube(Vector3 {x: voxel.x as f32 + 0.5, y: voxel.y as f32 + 0.5, z: voxel.z as f32 + 0.5}, 1.0, 1.0, 1.0, voxel.color);
    }
}

// more functionaloty for the camera
trait SpellCoderCamera {
    fn get_look_direction(&self) -> Vector3;
    fn get_look_direction_vec2(&self) -> Vector2;
    fn set_look_direction(&mut self, dir: Vector3);
    fn set_look_direction_vec2(&mut self, dir: Vector2);
    fn move_camera(&mut self, delta: Vector3);
}
impl SpellCoderCamera for Camera3D {
    // get normalized vector of the direction of the camera
    fn get_look_direction(&self) -> Vector3 { 
        (self.target - self.position).normalized()
    }

    // set direction of the camera
    fn set_look_direction(&mut self, dir: Vector3) { 
        self.target = self.position + dir;
    }

    // get direction of the camera as yaw & pitch
    fn get_look_direction_vec2(&self) -> Vector2 { 
        let vec3 = self.get_look_direction();
        let h_vec2 = Vector2 {x: vec3.x, y: vec3.z}.normalized();
        Vector2 { x: h_vec2.y.acos()*h_vec2.x.signum(), y: vec3.y.asin() }
    }

    // set direction of the camera as yaw & pitch
    fn set_look_direction_vec2(&mut self, dir: Vector2) { 
        let h_vec2 = Vector2 {x: dir.x.sin(), y: dir.x.cos()}.normalized() * dir.y.cos().abs();
        self.set_look_direction(
            Vector3 { x: h_vec2.x, y: dir.y.sin(), z: h_vec2.y }.normalized()
        );
    }

    // move camera without changing yaw & pitch
    fn move_camera(&mut self, delta: Vector3) {
        self.position += delta;
        self.target += delta;
    }
}

fn process_input(rl: &mut RaylibHandle, camera: &mut Camera3D) {
    let mdelta = rl.get_mouse_delta(); // get mouse input
    let cam_angle = camera.get_look_direction_vec2(); // get current yaw & pitch

    if (mdelta != Vector2 {x: 0.0, y: 0.0}) {
        let new_cam_angle = Vector2 { x: cam_angle.x -mdelta.x/180.0*PI, y: cam_angle.y-(mdelta.y/180.0*PI) }; // construct new yaw & pitch vector
        camera.set_look_direction_vec2(new_cam_angle);
    }

}

fn main() {
    // set up window
    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("Spellcoder")
        .build();
    rl.set_target_fps(60);
    rl.disable_cursor();

    // set up camera
    let mut camera = Camera3D::perspective(
        Vector3 {x: 3.0, y: 2.0, z: 3.0},
        Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        Vector3 {x: 0.0, y: 1.0, z: 0.0},
        70.0
    );
    camera.set_look_direction_vec2(Vector2 { x: -3.0*PI/4.0, y: -0.5 });

    // mainloop
    while !rl.window_should_close() {
        let time = rl.get_time() as f32;
        
        process_input(&mut rl, &mut camera);

        // set up drawing
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // use d for 2d drawing here (background)

        let mut d3d = d.begin_mode3D(camera);
        
        // use d3d for 3d drawing here
        d3d.draw_grid(10, 1.0);
        d3d.draw_voxel(Voxel {x: 0, y: 0, z: 0, color: Color::from_hex("0080FF").unwrap().into()});
        d3d.draw_voxel(Voxel {x: 2, y: 0, z: 0, color: Color::from_hex("FF8000").unwrap().into()});

        drop(d3d);
        // use d for 2d drawing here (overlay)

        d.draw_fps(10, 10);
    }
}
