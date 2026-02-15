use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct S {
    #[ply(count = "u8")]
    a: u32,
}

fn main() {}
