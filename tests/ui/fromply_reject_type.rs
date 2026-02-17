use ply_rs_bw::FromPly;

#[derive(FromPly)]
struct S {
    #[ply(type = "float")]
    a: Vec<u32>,
}

fn main() {}
