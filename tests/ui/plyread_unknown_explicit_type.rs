use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Element {
    #[ply(type = "uinit")]
    foo: u32,
}

fn main() {}
