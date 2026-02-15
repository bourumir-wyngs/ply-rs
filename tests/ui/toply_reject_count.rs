use ply_rs_bw::ToPly;

#[derive(ToPly)]
struct S {
    #[ply(count = "u8")]
    a: Vec<u32>,
}

fn main() {}
