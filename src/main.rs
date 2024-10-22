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

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // use d for 2d drawing here (background)

        camera.position.x = time.cos() * 3.0;
        camera.position.z = time.sin() * 3.0;

        let mut d3d = d.begin_mode3D(camera);
        
        // use d3d for 3d drawing here
        d3d.draw_voxel(Voxel {x: 0, y: 0, z: 0, color: Color::from_hex("0080FF").unwrap().into()});
        d3d.

        drop(d3d);
        // use d for 2d drawing here (overlay)

        d.draw_fps(10, 10);
    }
}
