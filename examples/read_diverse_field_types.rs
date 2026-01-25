
use ply_rs_bw::parser::Parser;
use ply_rs_bw::ply::{DefaultElement, Property};
use std::fs::File;
use std::io::BufReader;

/// Loads a mesh from a file, returning a list of vertices (x, y, z represented as `f32`)
/// and triangle indices (indices represented as `u32`).
///
/// This example focuses on real-world scenarios where fields may be represented using
/// different types (e.g., `f32` vs `f64`, `u32` vs `i32`, etc.). To read PLY data
/// from various sources, the reader must adapt to the types used in the input.
pub fn read_mesh(ply_file_path: &str) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    // Open the file
    let file = File::open(ply_file_path)
        .unwrap_or_else(|_| panic!("Could not open PLY file: {}", ply_file_path));
    let mut reader = BufReader::new(file);

    // Create a PLY parser and parse the header
    let parser = Parser::<DefaultElement>::new();
    let ply = parser
        .read_ply(&mut reader)
        .unwrap_or_else(|_| panic!("Could not parse PLY file: {}", ply_file_path));

    // Extract vertices and faces from the PLY file
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // PLY may have internally different types.
    if let Some(vertices_elem) = ply.payload.get("vertex") {
        for vertex in vertices_elem {
            let x = match vertex.get("x").unwrap() {
                Property::Float(val) => *val,
                Property::Double(val) => *val as f32,
                _ => panic!("Unexpected type for vertex x"),
            };
            let y = match vertex.get("y").unwrap() {
                Property::Float(val) => *val,
                Property::Double(val) => *val as f32,
                _ => panic!("Unexpected type for vertex y"),
            };
            let z = match vertex.get("z").unwrap() {
                Property::Float(val) => *val,
                Property::Double(val) => *val as f32,
                _ => panic!("Unexpected type for vertex z"),
            };

            vertices.push([x, y, z]); // Push vertex to the vertices list
        }
    }

    // Extract faces (indices)
    fn extract_indices<T>(indices_list: &[T], i: usize) -> [u32; 3]
    where
        T: TryInto<u32> + Copy,
        <T as TryInto<u32>>::Error: std::fmt::Debug,
    {
        if indices_list.len() < 3 {
            panic!("Insufficient indices for a triangle in face {}", i);
        }
        const X: &str = "Failed to convert triangle index to u32";
        [
            indices_list[0].try_into().expect(X),
            indices_list[1].try_into().expect(X),
            indices_list[2].try_into().expect(X),
        ]
    }

    if let Some(faces_elem) = ply.payload.get("face") {
        for (i, face) in faces_elem.iter().enumerate() {
            match face.get("vertex_indices") {
                Some(Property::ListUInt(indices_list)) => {
                    indices.push(extract_indices(indices_list, i));
                }
                Some(Property::ListInt(indices_list)) => {
                    indices.push(extract_indices(indices_list, i));
                }
                Some(Property::ListUShort(indices_list)) => {
                    indices.push(extract_indices(indices_list, i));
                }
                Some(Property::ListShort(indices_list)) => {
                    indices.push(extract_indices(indices_list, i));
                }
                Some(property) => {
                    panic!("Unexpected property type for face {}: {:?}", i, property);
                }
                None => {
                    panic!("Missing 'vertex_indices' for face {}", i);
                }
            }
        }
    } else {
        panic!("No 'face' element in payload");
    }

    (vertices, indices)
}

/// Helper function to print the results. Examples used only contain a single triangle.
/// Python script used for generating test data can be found in the folder 'scripts'
fn print_ply_data(data: (Vec<[f32; 3]>, Vec<[u32; 3]>)) {
    let (vertices, indices) = data;

    println!("Vertices:");
    for (i, vertex) in vertices.iter().enumerate() {
        println!("  {}: [{:.3}, {:.3}, {:.3}]", i, vertex[0], vertex[1], vertex[2]);
    }

    println!("Indices:");
    for (i, index) in indices.iter().enumerate() {
        println!("  {}: [{}, {}, {}]", i, index[0], index[1], index[2]);
    }
}


fn main() {
    // The output from four lines below must be identical.
    print_ply_data(read_mesh("example_plys/diverse_field_formats/doubles_ints.ply"));
    print_ply_data(read_mesh("example_plys/diverse_field_formats/doubles_shorts.ply"));
    print_ply_data(read_mesh("example_plys/diverse_field_formats/floats_ints.ply"));
    print_ply_data(read_mesh("example_plys/diverse_field_formats/floats_shorts.ply"));
}