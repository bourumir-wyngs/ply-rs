use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Element {
    // Signed/unsigned mismatch should still be rejected.
    #[ply(type = "uint")]
    foo: i64,
}

fn main() {}
