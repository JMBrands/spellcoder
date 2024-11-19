
fn main() {
    cc::Build::new()
        .file("src/mesh.c")
        .include("lib/raylib-5.0_linux_i386/include")
        .include("src")
        .compile("mesh");
    
}