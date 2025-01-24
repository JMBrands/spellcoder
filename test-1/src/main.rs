#[repr(C)]
#[derive(Clone, Copy, Debug)]
enum S {
    A
}

fn main() {
    println!("{:#?}", [[[S::A; 16]; 65536]; 16]);
}
