use ply_rs_bw::{PlyRead, ToPly};

#[derive(PlyRead, Default)]
struct Vertex {
    x: f32,
}

#[derive(ToPly)]
struct Payload {
    #[ply(name = "vertex,vert")]
    vertices: Vec<Vertex>,
    #[ply(name = "edge,vert")]
    edges: Vec<Vertex>,
}

fn main() {}
