use ply_rs_bw::{PlyRead, FromPly};

#[derive(Debug, PlyRead, PartialEq, Default)]
struct Vertex {
    #[ply(name = "x, X")]
    x: f32,
    #[ply(name = "y, Y")]
    y: f32,
    #[ply(name = "z, Z")]
    z: f32,
}

#[derive(Debug, FromPly, PartialEq)]
struct Mesh {
    #[ply(name = "vertex, vertices")]
    vertices: Vec<Vertex>,
}

#[test]
fn test_synonyms_property() {
    // PLY with "X", "Y", "Z" instead of "x", "y", "z"
    // And "vertex" element
    let txt = b"ply\n\
format ascii 1.0\n\
element vertex 1\n\
property float X\n\
property float Y\n\
property float Z\n\
end_header\n\
1.0 2.0 3.0\n";
    
    let mut reader = std::io::Cursor::new(txt);
    let mesh = Mesh::read_ply(&mut reader).unwrap();
    
    assert_eq!(mesh.vertices.len(), 1);
    assert_eq!(mesh.vertices[0].x, 1.0);
    assert_eq!(mesh.vertices[0].y, 2.0);
    assert_eq!(mesh.vertices[0].z, 3.0);
}

#[test]
fn test_synonyms_element() {
    // PLY with "vertices" instead of "vertex"
    // And "x", "y", "z" properties
    let txt = b"ply\n\
format ascii 1.0\n\
element vertices 1\n\
property float x\n\
property float y\n\
property float z\n\
end_header\n\
1.0 2.0 3.0\n";
    
    let mut reader = std::io::Cursor::new(txt);
    let mesh = Mesh::read_ply(&mut reader).unwrap();
    
    assert_eq!(mesh.vertices.len(), 1);
    assert_eq!(mesh.vertices[0].x, 1.0);
}

#[test]
fn test_synonyms_mixed() {
    // PLY with "vertices" instead of "vertex"
    // And "X", "Y", "Z" instead of "x", "y", "z"
    let txt = b"ply\n\
format ascii 1.0\n\
element vertices 1\n\
property float X\n\
property float Y\n\
property float Z\n\
end_header\n\
1.0 2.0 3.0\n";
    
    let mut reader = std::io::Cursor::new(txt);
    let mesh = Mesh::read_ply(&mut reader).unwrap();
    
    assert_eq!(mesh.vertices.len(), 1);
    assert_eq!(mesh.vertices[0].x, 1.0);
}
