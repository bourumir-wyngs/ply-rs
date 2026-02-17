use ply_rs_bw::{FromPly, PlyRead};

#[derive(PlyRead, Default)]
struct Vertex {
    x: f32,
}

#[derive(FromPly)]
struct Payload {
    #[ply(name = "vertex,vert")]
    vertices: Vec<Vertex>,
    #[ply(name = "face,vert")]
    faces: Vec<Vertex>,
}

fn main() {}
