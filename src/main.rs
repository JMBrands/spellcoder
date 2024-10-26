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
    fn draw_voxel(&mut self, voxel: Voxel) {
        self.draw_cube(Vector3 {x: voxel.x as f32, y: voxel.y as f32, z: voxel.z as f32}, 1.0, 1.0, 1.0, voxel.color);
    }
}

trait SpellCoderCamera {
    fn get_look_direction(&self) -> Vector3;
    fn get_look_direction_vec2(&self) -> Vector2;
    fn set_look_direction(&mut self, dir: Vector3);
    fn set_look_direction_vec2(&mut self, dir: Vector2);
    fn move_camera(&mut self, delta: Vector3);
}
impl SpellCoderCamera for Camera3D {
    fn get_look_direction(&self) -> Vector3 {
        (self.target - self.position).normalized()
    }

    fn set_look_direction(&mut self, dir: Vector3) {
        self.target = self.position + dir;
    }
    
    fn get_look_direction_vec2(&self) -> Vector2 {
        let vec3 = self.get_look_direction();
        let h_vec2 = Vector2 {x: vec3.x, y: vec3.z}.normalized();
        Vector2 { x: h_vec2.y.acos()*h_vec2.x.signum(), y: vec3.y.asin() }
    }
    
    fn set_look_direction_vec2(&mut self, dir: Vector2) {
        let h_vec2 = Vector2 {x: dir.x.sin(), y: dir.x.cos()}.normalized() * (dir.y/(2.0*PI)).cos().abs();
        self.set_look_direction(
            Vector3 { x: h_vec2.x, y: (dir.y/(2.0*PI)).sin(), z: h_vec2.y }
        );
    }

    fn move_camera(&mut self, delta: Vector3) {
        self.position += delta;
        self.target += delta;
    }
}

fn process_input(rl: &mut RaylibHandle, camera: &mut Camera3D) {
    let mdelta = rl.get_mouse_delta();
    let cam_angle = camera.get_look_direction_vec2();
    if (mdelta != Vector2 {x: 0.0, y: 0.0}) {
        let new_cam_angle = cam_angle + Vector2 { x: -mdelta.x/180.0*PI, y: -mdelta.y/180.0*PI };
        camera.set_look_direction_vec2(new_cam_angle);
        println!("{:#?}", camera.get_look_direction());
    }

}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("Spellcoder")
        .build();

    let mut camera = Camera3D::perspective(
        Vector3 {x: 3.0, y: 2.0, z: 3.0},
        Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        Vector3 {x: 0.0, y: 1.0, z: 0.0},
        70.0
    );

    rl.set_target_fps(60);
    rl.disable_cursor();
    while !rl.window_should_close() {
        let time = rl.get_time() as f32;
        
        process_input(&mut rl, &mut camera);
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // use d for 2d drawing here (background)

        // camera.position.x = time.cos() * 3.0;
        // camera.position.z = time.sin() * 3.0;
        let mut d3d = d.begin_mode3D(camera);
        
        // use d3d for 3d drawing here
        d3d.draw_grid(50, 1.0);
        d3d.draw_voxel(Voxel {x: 0, y: 0, z: 0, color: Color::from_hex("0080FF").unwrap().into()});

        drop(d3d);
        // use d for 2d drawing here (overlay)

        d.draw_fps(10, 10);
    }
}
