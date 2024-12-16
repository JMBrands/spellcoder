
fn main() {
    cc::Build::new()
        .file("src/chunkmesh.c")
        .include("lib/raylib/include/")
        .compile("chunkmesh.a");
}