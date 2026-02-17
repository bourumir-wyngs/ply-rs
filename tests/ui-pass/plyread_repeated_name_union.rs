use ply_rs_bw::PlyRead;

#[derive(PlyRead)]
struct Vertex {
    // The `name` sub-attribute may be specified repeatedly; all values are collected.
    #[ply(name = "x")]
    #[ply(name = "pos_x, x_position")]
    x: f32,
}

fn main() {}
