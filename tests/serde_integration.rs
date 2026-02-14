use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Face {
    vertex_index: Vec<i32>,
}

// A standard structure for PLY file
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Mesh {
    vertex: Vec<Vertex>,
    face: Vec<Face>,
}

#[test]
fn test_serde_read_simple() {
    let ply_data = "ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
element face 1
property list uchar int vertex_index
end_header
0.1 0.2 0.3
0.4 0.5 0.6
3 0 1 0
";

    // This function doesn't exist yet
    let mesh: Mesh = ply_rs_bw::from_reader(ply_data.as_bytes()).unwrap();

    assert_eq!(mesh.vertex.len(), 2);
    assert_eq!(mesh.vertex[0].x, 0.1);
    assert_eq!(mesh.vertex[1].y, 0.5);
    assert_eq!(mesh.face.len(), 1);
    assert_eq!(mesh.face[0].vertex_index, vec![0, 1, 0]);
}

#[test]
fn test_serde_write_simple() {
    let vertex = vec![
        Vertex { x: 0.1, y: 0.2, z: 0.3 },
        Vertex { x: 0.4, y: 0.5, z: 0.6 },
    ];
    let face = vec![
        Face { vertex_index: vec![0, 1, 0] },
    ];
    let mesh = Mesh { vertex, face };

    let mut buf = Vec::new();
    ply_rs_bw::to_writer(&mut buf, &mesh).unwrap();

    let output = String::from_utf8(buf).unwrap();
    println!("Output PLY:\n{}", output);

    // Verify by reading back
    let mesh_read: Mesh = ply_rs_bw::from_reader(output.as_bytes()).unwrap();
    assert_eq!(mesh, mesh_read);
}

mod rename_tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Vertex {
        #[serde(rename = "x_coord")]
        x: f32,
        #[serde(rename = "y_coord")]
        y: f32,
        #[serde(rename = "z_coord")]
        z: f32,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Face {
        #[serde(rename = "vertex_indices")]
        indices: Vec<i32>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Mesh {
        #[serde(rename = "vertex")]
        vertices: Vec<Vertex>,
        #[serde(rename = "face")]
        faces: Vec<Face>,
    }

    #[test]
    fn test_serde_rename_read() {
        let ply_data = "ply
format ascii 1.0
element vertex 2
property float x_coord
property float y_coord
property float z_coord
element face 1
property list uchar int vertex_indices
end_header
0.1 0.2 0.3
0.4 0.5 0.6
3 0 1 0
";

        let mesh: Mesh = ply_rs_bw::from_reader(ply_data.as_bytes()).unwrap();

        assert_eq!(mesh.vertices.len(), 2);
        assert_eq!(mesh.vertices[0].x, 0.1);
        assert_eq!(mesh.vertices[1].y, 0.5);
        assert_eq!(mesh.faces.len(), 1);
        assert_eq!(mesh.faces[0].indices, vec![0, 1, 0]);
    }

    #[test]
    fn test_serde_rename_write() {
        let vertices = vec![
            Vertex { x: 0.1, y: 0.2, z: 0.3 },
            Vertex { x: 0.4, y: 0.5, z: 0.6 },
        ];
        let faces = vec![
            Face { indices: vec![0, 1, 0] },
        ];
        let mesh = Mesh { vertices, faces };

        let mut buf = Vec::new();
        ply_rs_bw::to_writer(&mut buf, &mesh).unwrap();

        let output = String::from_utf8(buf).unwrap();
        println!("Output PLY:\n{}", output);

        // Check header for renamed elements and properties
        assert!(output.contains("element vertex 2"));
        assert!(output.contains("property float x_coord"));
        assert!(output.contains("property float y_coord"));
        assert!(output.contains("property float z_coord"));
        assert!(output.contains("element face 1"));
        assert!(output.contains("property list uchar int vertex_indices"));

        // Verify by reading back
        let mesh_read: Mesh = ply_rs_bw::from_reader(output.as_bytes()).unwrap();
        assert_eq!(mesh, mesh_read);
    }
}
