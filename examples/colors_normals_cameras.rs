/// This example demonstrates how to read complex data beyond basic vertex positions:
/// colors, normals and positioning of multiple cameras.
use ply_rs_bw::{FromPly, PlyRead, PlyWrite, ToPly};

/// A vertex structure containing position, normal, texture coordinates, and color data.
#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct Vertex {
    /// X coordinate of the vertex position
    x: f32,
    /// Y coordinate of the vertex position
    y: f32,
    /// Z coordinate of the vertex position
    z: f32,
    /// X component of the normal vector
    nx: f32,
    /// Y component of the normal vector
    ny: f32,
    /// Z component of the normal vector
    nz: f32,
    /// Texture coordinate u
    u: f32,
    /// Texture coordinate v
    v: f32,
    /// Red color component (0-255)
    red: u8,
    /// Green color component (0-255)
    green: u8,
    /// Blue color component (0-255)
    blue: u8,
    /// Alpha transparency component (0-255)
    alpha: u8,
}

/// A face structure representing a polygon, typically a triangle or quad.
#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct Face {
    /// List of indices pointing to the vertices that form this face.
    /// The property name "vertex_indices" in the PLY file is mapped to this field.
    #[ply(name = "vertex_indices")]
    vertex_indices: Vec<i32>,
}

/// A camera structure containing extrinsic (position, orientation) and intrinsic parameters.
#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct Camera {
    /// X coordinate of the camera position
    px: f32,
    /// Y coordinate of the camera position
    py: f32,
    /// Z coordinate of the camera position
    pz: f32,
    /// X component of the orientation quaternion
    qx: f32,
    /// Y component of the orientation quaternion
    qy: f32,
    /// Z component of the orientation quaternion
    qz: f32,
    /// W (scalar) component of the orientation quaternion
    qw: f32,
    /// Focal length in X direction
    fx: f32,
    /// Focal length in Y direction
    fy: f32,
    /// Principal point X coordinate
    cx: f32,
    /// Principal point Y coordinate
    cy: f32,
}

/// A mesh structure that aggregates all elements (vertices, faces, cameras) read from the PLY file.
#[derive(Debug, FromPly, ToPly, Clone, PartialEq)]
struct Mesh {
    /// Collection of vertices.
    /// Maps to the "vertex" element in the PLY file.
    #[ply(name = "vertex")]
    vertices: Vec<Vertex>,
    /// Collection of faces.
    /// Maps to the "face" element in the PLY file.
    #[ply(name = "face")]
    faces: Vec<Face>,
    /// Collection of cameras.
    /// Maps to the "camera" element in the PLY file.
    #[ply(name = "camera")]
    cameras: Vec<Camera>,
}

fn main() {
    let path = "example_plys/colors_normals_cameras.ply";
    let f = std::fs::File::open(path).unwrap();
    let mut f = std::io::BufReader::new(f);

    // Read the ASCII PLY
    let mesh = Mesh::read_ply(&mut f);

    match mesh {
        Ok(m) => {
            println!("Read PLY successfully:");
            println!("Vertices: {}", m.vertices.len());
            for (i, v) in m.vertices.iter().enumerate() {
                println!("Vertex {}: {:?}", i, v);
            }
            println!("Faces: {}", m.faces.len());
            for (i, f) in m.faces.iter().enumerate() {
                println!("Face {}: {:?}", i, f);
            }
            println!("Cameras: {}", m.cameras.len());
            for (i, c) in m.cameras.iter().enumerate() {
                println!("Camera {}: {:?}", i, c);
            }

            // Write the same mesh in binary little-endian PLY format.
            // This is the most common binary PLY encoding.
            let mut buf_binary = Vec::<u8>::new();
            let written = m
                .write_ply_with_encoding(
                    &mut buf_binary,
                    ply_rs_bw::ply::Encoding::BinaryLittleEndian,
                )
                .unwrap();
            println!("{} bytes written (Binary Little Endian)", written);

            // Optional round-trip check: read the binary output back and compare.
            let mut cursor = std::io::Cursor::new(&buf_binary);
            let read_back = Mesh::read_ply(&mut cursor).unwrap();
            assert_eq!(m, read_back);
            println!("Verification successful: read-back mesh matches written mesh.");
        }
        Err(e) => println!("Error reading PLY: {:?}", e),
    }
}
