use ply_rs_bw::PlyRead;

// This should NOT require `Default` on the struct itself.
// Only the fields need to implement `Default` for `PlyRead`'s generated `new()`.

#[derive(PlyRead)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

fn main() {}
