use ply_rs_bw::PlyWrite;

#[derive(PlyWrite)]
struct V {
    #[ply(type = "float")]
    foo: Vec<u32>,
}

fn main() {}
