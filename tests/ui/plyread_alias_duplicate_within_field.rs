use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Element {
    #[ply(name = "a,a")]
    x: i32,
}

fn main() {}
