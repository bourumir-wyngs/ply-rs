use ply_rs_bw::PlyWrite;

#[derive(PlyWrite)]
struct V {
    #[ply(type = "float")]
    foo: u32,
}

fn main() {}
