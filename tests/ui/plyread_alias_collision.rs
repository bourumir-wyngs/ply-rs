use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Element {
    #[ply(name = "a,b")]
    x: i32,
    #[ply(name = "c,b")]
    y: i32,
}

fn main() {}
