use ply_rs_bw::{PlyAccess, ToPly, FromPly};

#[derive(Debug, Default, PlyAccess, Clone, PartialEq)]
struct Vertex {
    #[ply(name = "x")]
    x: f32,
    #[ply(name = "y")]
    y: f32,
    #[ply(name = "z")]
    z: f32,
}

#[derive(Debug, Default, PlyAccess, Clone, PartialEq)]
struct Face {
    #[ply(name = "vertex_indices")]
    indices: Vec<i32>,
}

#[derive(Debug, ToPly, FromPly, PartialEq)]
struct Mesh {
    #[ply(name = "vertex")]
    vertices: Vec<Vertex>,
    #[ply(name = "face")]
    faces: Vec<Face>,
}

#[test]
#[cfg(not(miri))]
fn test_write_read_tetrahedron_macros() {
    // Create mesh
    let vertices = vec![
        Vertex { x: 1.0, y: 1.0, z: 1.0 },
        Vertex { x: 1.0, y: -1.0, z: -1.0 },
        Vertex { x: -1.0, y: 1.0, z: -1.0 },
        Vertex { x: -1.0, y: -1.0, z: 1.0 },
    ];

    let faces = vec![
        Face { indices: vec![0, 1, 2] },
        Face { indices: vec![0, 3, 1] },
        Face { indices: vec![0, 2, 3] },
        Face { indices: vec![1, 3, 2] },
    ];

    let mesh = Mesh { vertices, faces };

    // Write
    let mut buf = Vec::<u8>::new();
    mesh.write_ply(&mut buf).unwrap();

    // Read back
    let mut cursor = std::io::Cursor::new(buf);
    let read_mesh = Mesh::read_ply(&mut cursor).unwrap();

    // Assert
    assert_eq!(mesh, read_mesh);
}
