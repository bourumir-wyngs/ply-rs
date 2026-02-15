use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Element {
    // The file stores this as a 32-bit `int`, but the Rust field can be wider.
    #[ply(type = "int")]
    a: i64,

    #[ply(type = "int")]
    b: Option<i64>,

    #[ply(type = "uint")]
    c: u128,

    #[ply(type = "float")]
    d: f64,

    #[ply(type = "int")]
    e: Vec<i64>,
}

fn main() {}
