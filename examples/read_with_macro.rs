use ply_rs_bw::{FromPly, PlyRead};
use std::fs::File;

#[derive(Debug, Default, PlyRead)]
struct Vertex {
    // we use maximal abstraction (ply types and names are inferred).
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Default, PlyRead)]
struct Face {
    // we use maximum details
    #[ply(name = "vertex_indices", type = "uint")]
    indices: Vec<u32>,
}

#[derive(Debug, FromPly)]
struct Mesh {
    #[ply(name = "vertex")]
    vertices: Vec<Vertex>,
    #[ply(name = "face")]
    faces: Vec<Face>,
}

fn read_mesh(ply_file_path: &str) -> Mesh {
    let mut file = File::open(ply_file_path)
        .unwrap_or_else(|_| panic!("Could not open PLY file: {}", ply_file_path));

    Mesh::read_ply(&mut file)
        .unwrap_or_else(|_| panic!("Could not parse PLY file: {}", ply_file_path))
}

fn print_ply_data(mesh: &Mesh) {
    println!("Vertices:");
    for (i, v) in mesh.vertices.iter().enumerate() {
        println!("  {}: [{:.3}, {:.3}, {:.3}]", i, v.x, v.y, v.z);
    }

    println!("Indices:");
    for (i, f) in mesh.faces.iter().enumerate() {
        if f.indices.len() < 3 {
            println!("  {}: <insufficient indices>", i);
        } else {
            println!("  {}: [{}, {}, {}]", i, f.indices[0], f.indices[1], f.indices[2]);
        }
    }
}

fn main() {
    let mesh = read_mesh("example_plys/diverse_field_formats/doubles_ints.ply");
    print_ply_data(&mesh);
    let mesh = read_mesh("example_plys/diverse_field_formats/doubles_shorts.ply");
    print_ply_data(&mesh);
    let mesh = read_mesh("example_plys/diverse_field_formats/floats_ints.ply");
    print_ply_data(&mesh);
    let mesh = read_mesh("example_plys/diverse_field_formats/floats_shorts.ply");
    print_ply_data(&mesh);
}
