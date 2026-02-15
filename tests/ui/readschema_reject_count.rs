use ply_rs_bw::ReadSchema;

#[derive(ReadSchema)]
struct S {
    #[ply(count = "u8")]
    a: u32,
}

fn main() {}
