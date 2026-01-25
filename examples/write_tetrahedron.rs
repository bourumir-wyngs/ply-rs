use ply_rs_bw::ply::{
    Addable, DefaultElement, ElementDef, Encoding, Ply, Property, PropertyDef, PropertyType,
    ScalarType,
};
use ply_rs_bw::writer::Writer;

/// Demonstrates writing a PLY file representing a tetrahedron.
///
/// A tetrahedron is a pyramid-like shape, consisting of 4 vertices and 4 triangular faces.
/// This example shows how to define vertex positions and face indices using the PLY format.
fn main() {
    // Set up a target buffer (could also be a file)
    let mut buf = Vec::<u8>::new();

    // Create a ply object with tetrahedron data
    let mut ply = create_tetrahedron_ply();

    // Set up a writer and write the PLY data
    let w = Writer::new();
    let written = w.write_ply(&mut buf, &mut ply).unwrap();
    println!("{} bytes written", written);

    // Display the written PLY file content
    let output = String::from_utf8(buf).unwrap();
    println!("Written PLY data:\n{}", output);

    // Demonstrate binary writing (skipped under Miri)
    #[cfg(not(miri))]
    {
        let binary_buf = write_ply_binary(&ply);
        println!("\nBinary PLY: {} bytes written", binary_buf.len());
    }
}

/// Writes a PLY object in binary little endian format (the most popular binary format).
///
/// Binary little endian is the most widely supported binary PLY format as it matches
/// the native byte order of most modern systems (x86, x64, ARM).
///
/// # Arguments
/// * `ply` - The PLY object to write
///
/// # Returns
/// A byte vector containing the binary PLY data
#[cfg(not(miri))]
fn write_ply_binary(ply: &Ply<DefaultElement>) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    let mut ply_binary = ply.clone();
    ply_binary.header.encoding = Encoding::BinaryLittleEndian;

    let w = Writer::new();
    w.write_ply(&mut buf, &mut ply_binary).unwrap();
    buf
}

/// Creates a PLY object representing a regular tetrahedron.
///
/// The tetrahedron is centered roughly at the origin with vertices at:
/// - (1, 1, 1)
/// - (1, -1, -1)
/// - (-1, 1, -1)
/// - (-1, -1, 1)
fn create_tetrahedron_ply() -> Ply<DefaultElement> {
    let mut ply = Ply::<DefaultElement>::new();
    ply.header.encoding = Encoding::Ascii;
    ply.header.comments.push("Tetrahedron example".to_string());

    // Define vertex element with x, y, z properties
    let mut vertex_element = ElementDef::new("vertex".to_string());
    vertex_element
        .properties
        .add(PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Float)));
    vertex_element
        .properties
        .add(PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::Float)));
    vertex_element
        .properties
        .add(PropertyDef::new("z".to_string(), PropertyType::Scalar(ScalarType::Float)));
    ply.header.elements.add(vertex_element);

    // Define face element with vertex_indices list property
    let mut face_element = ElementDef::new("face".to_string());
    face_element.properties.add(PropertyDef::new(
        "vertex_indices".to_string(),
        PropertyType::List(ScalarType::UChar, ScalarType::Int),
    ));
    ply.header.elements.add(face_element);

    // Add the 4 vertices of a regular tetrahedron
    let vertices = vec![
        [1.0_f32, 1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
    ];

    let mut vertex_list = Vec::new();
    for [x, y, z] in vertices {
        let mut vertex = DefaultElement::new();
        vertex.insert("x".to_string(), Property::Float(x));
        vertex.insert("y".to_string(), Property::Float(y));
        vertex.insert("z".to_string(), Property::Float(z));
        vertex_list.push(vertex);
    }
    ply.payload.insert("vertex".to_string(), vertex_list);

    // Add the 4 triangular faces
    // Each face is defined by 3 vertex indices (counter-clockwise winding for outward normals)
    let faces = vec![
        vec![0, 1, 2], // Face 1
        vec![0, 3, 1], // Face 2
        vec![0, 2, 3], // Face 3
        vec![1, 3, 2], // Face 4
    ];

    let mut face_list = Vec::new();
    for indices in faces {
        let mut face = DefaultElement::new();
        face.insert(
            "vertex_indices".to_string(),
            Property::ListInt(indices),
        );
        face_list.push(face);
    }
    ply.payload.insert("face".to_string(), face_list);

    ply
}
