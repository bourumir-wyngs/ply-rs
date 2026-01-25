use ply_rs_bw::*;
use ply_rs_bw::ply::*;
use std::io::{ Read, BufReader };

type Ply = ply::Ply<DefaultElement>;

fn read_buff<T: Read>(mut buf: &mut T) -> Ply {
    let p = parser::Parser::new();
    let ply = p.read_ply(&mut buf);
    assert!(ply.is_ok(), "{}", format!("failed: {}", ply.err().unwrap()));
    ply.unwrap()
}

fn write_buff(ply: &Ply) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    let w = writer::Writer::new();
    w.write_ply_unchecked(&mut buf, ply).unwrap();
    buf
}

fn read_write_ply(ply: &Ply) -> Ply {
    println!("writing ply:\n{:?}", ply);
    let ve : Vec<u8> = write_buff(ply);
    let txt = String::from_utf8(ve.clone()).unwrap();
    println!("written ply:\n{}", txt);
    let mut buff = BufReader::new(&(*ve));
    let new_ply = read_buff(&mut buff);
    println!("read ply:\n{:?}", new_ply);
    assert_eq!(ply.header, new_ply.header);
    assert_eq!(ply.payload, new_ply.payload);
    new_ply
}

fn create_min() -> Ply {
    let mut ply = Ply::new();
    assert!(ply.make_consistent().is_ok());
    ply
}

fn create_basic_header() -> Ply {
    let mut ply = Ply::new();
    let p = PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Int));
    let mut e = ElementDef::new("point".to_string());
    e.properties.add(p);
    let c = "Hi, I'm your friendly comment.".to_string();
    let oi = "And I'm your object information.".to_string();
    ply.header.elements.add(e);
    ply.header.comments.push(c);
    ply.header.obj_infos.push(oi);
    assert!(ply.make_consistent().is_ok());
    ply
}

fn create_single_elements() -> Ply {
    let mut ply = Ply::new();

    let mut e = ElementDef::new("point".to_string());
    let p = PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Int));
    e.properties.add(p);
    let p = PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::UInt));
    e.properties.add(p);

    let mut list = Vec::new();
    let mut pe = KeyMap::new();
    pe.insert("x".to_string(), Property::Int(-7));
    pe.insert("y".to_string(), Property::UInt(5));
    list.push(pe);
    let mut pe = KeyMap::new();
    pe.insert("x".to_string(), Property::Int(2));
    pe.insert("y".to_string(), Property::UInt(4));
    list.push(pe);
    ply.payload.insert("point".to_string(), list);

    let c = "Hi, I'm your friendly comment.".to_string();
    let oi = "And I'm your object information.".to_string();
    ply.header.elements.add(e);
    ply.header.comments.push(c);
    ply.header.obj_infos.push(oi);
    assert!(ply.make_consistent().is_ok());
    ply
}
fn create_list_elements() -> Ply {
    let mut ply = Ply::new();

    let mut e = ElementDef::new("aList".to_string());
    let p = PropertyDef::new("x".to_string(), PropertyType::List(ScalarType::Int, ScalarType::Int));
    e.properties.add(p);

    let mut list = Vec::new();
    let mut pe = KeyMap::new();
    pe.insert("x".to_string(), Property::ListInt(vec![-7, 17, 38]));
    list.push(pe);
    let mut pe = KeyMap::new();
    pe.insert("x".to_string(), Property::ListInt(vec![13, -19, 8, 33]));
    list.push(pe);
    ply.payload.insert("aList".to_string(), list);

    let c = "Hi, I'm your friendly comment.".to_string();
    let oi = "And I'm your object information.".to_string();
    ply.header.elements.add(e);
    ply.header.comments.push(c);
    ply.header.obj_infos.push(oi);
    assert!(ply.make_consistent().is_ok());
    ply
}

#[test]
fn write_header_min() {
    let ply = create_min();
    let new_ply = read_write_ply(&ply);
    assert_eq!(ply, new_ply);
}
#[test]
fn write_basic_header() {
    let ply = create_basic_header();
    let new_ply = read_write_ply(&ply);
    assert_eq!(ply, new_ply);
}
#[test]
fn write_single_elements() {
    let ply = create_single_elements();
    let new_ply = read_write_ply(&ply);
    assert_eq!(ply, new_ply);
}
#[test]
fn write_list_elements() {
    let ply = create_list_elements();
    let new_ply = read_write_ply(&ply);
    assert_eq!(ply, new_ply);
}

// Helper function for binary write-read round-trip tests
fn read_write_binary_ply(ply: &Ply, encoding: Encoding) -> Ply {
    println!("writing ply with encoding {:?}:\n{:?}", encoding, ply);
    
    // Create a copy with the desired encoding
    let mut ply_to_write = ply.clone();
    ply_to_write.header.encoding = encoding;
    
    let ve: Vec<u8> = write_buff(&ply_to_write);
    println!("written {} bytes", ve.len());
    
    let mut buff = BufReader::new(&(*ve));
    let new_ply = read_buff(&mut buff);
    println!("read ply:\n{:?}", new_ply);
    
    // Compare header (encoding should match)
    assert_eq!(ply_to_write.header, new_ply.header);
    // Compare payload
    assert_eq!(ply_to_write.payload, new_ply.payload);
    new_ply
}

#[test]
fn write_binary_little_endian_single_elements() {
    let ply = create_single_elements();
    let new_ply = read_write_binary_ply(&ply, Encoding::BinaryLittleEndian);
    assert_eq!(ply.payload, new_ply.payload);
}

#[test]
fn write_binary_big_endian_single_elements() {
    let ply = create_single_elements();
    let new_ply = read_write_binary_ply(&ply, Encoding::BinaryBigEndian);
    assert_eq!(ply.payload, new_ply.payload);
}

#[test]
fn write_binary_little_endian_list_elements() {
    let ply = create_list_elements();
    let new_ply = read_write_binary_ply(&ply, Encoding::BinaryLittleEndian);
    assert_eq!(ply.payload, new_ply.payload);
}

#[test]
fn write_binary_big_endian_list_elements() {
    let ply = create_list_elements();
    let new_ply = read_write_binary_ply(&ply, Encoding::BinaryBigEndian);
    assert_eq!(ply.payload, new_ply.payload);
}

// ============================================================================
// Tetrahedron file tests - read pre-generated files and verify round-trip
// These tests use include_bytes! for miri compatibility (no file I/O at runtime)
// ============================================================================

/// Creates the expected tetrahedron PLY structure for comparison
fn create_tetrahedron_ply(encoding: Encoding) -> Ply {
    let mut ply = Ply::new();
    ply.header.encoding = encoding;
    ply.header.comments.push("Tetrahedron example".to_string());

    // Define vertex element with x, y, z properties
    let mut vertex_element = ElementDef::new("vertex".to_string());
    vertex_element.properties.add(PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Float)));
    vertex_element.properties.add(PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::Float)));
    vertex_element.properties.add(PropertyDef::new("z".to_string(), PropertyType::Scalar(ScalarType::Float)));
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
        let mut vertex = KeyMap::new();
        vertex.insert("x".to_string(), Property::Float(x));
        vertex.insert("y".to_string(), Property::Float(y));
        vertex.insert("z".to_string(), Property::Float(z));
        vertex_list.push(vertex);
    }
    ply.payload.insert("vertex".to_string(), vertex_list);

    // Add the 4 triangular faces
    let faces = vec![
        vec![0, 1, 2],
        vec![0, 3, 1],
        vec![0, 2, 3],
        vec![1, 3, 2],
    ];

    let mut face_list = Vec::new();
    for indices in faces {
        let mut face = KeyMap::new();
        face.insert("vertex_indices".to_string(), Property::ListInt(indices));
        face_list.push(face);
    }
    ply.payload.insert("face".to_string(), face_list);

    ply.make_consistent().unwrap();
    ply
}

#[test]
fn read_tetrahedron_ascii() {
    // To regenerate: cargo run --example write_tetrahedron
    // Then copy example_plys/tetrahedron_ascii.ply to the test location
    
    let file_bytes: &[u8] = include_bytes!("../example_plys/tetrahedron_ascii.ply");
    let mut reader = BufReader::new(file_bytes);
    let ply_read = read_buff(&mut reader);
    
    let expected = create_tetrahedron_ply(Encoding::Ascii);
    
    assert_eq!(expected.header, ply_read.header);
    assert_eq!(expected.payload, ply_read.payload);
    
    // Round-trip: write and read back
    let written = write_buff(&ply_read);
    let mut reader2 = BufReader::new(&written[..]);
    let ply_roundtrip = read_buff(&mut reader2);
    assert_eq!(ply_read, ply_roundtrip);
}

#[test]
fn read_tetrahedron_binary_little_endian() {
    // To regenerate: cargo run --example write_tetrahedron
    // Then copy example_plys/tetrahedron_little_endian.ply to the test location
    
    let file_bytes: &[u8] = include_bytes!("../example_plys/tetrahedron_little_endian.ply");
    let mut reader = BufReader::new(file_bytes);
    let ply_read = read_buff(&mut reader);
    
    let expected = create_tetrahedron_ply(Encoding::BinaryLittleEndian);
    
    assert_eq!(expected.header, ply_read.header);
    assert_eq!(expected.payload, ply_read.payload);
    
    // Round-trip: write and read back
    let written = write_buff(&ply_read);
    let mut reader2 = BufReader::new(&written[..]);
    let ply_roundtrip = read_buff(&mut reader2);
    assert_eq!(ply_read, ply_roundtrip);
}

#[test]
fn read_tetrahedron_binary_big_endian() {
    // To regenerate: cargo run --example write_tetrahedron
    // Then copy example_plys/tetrahedron_big_endian.ply to the test location
    
    let file_bytes: &[u8] = include_bytes!("../example_plys/tetrahedron_big_endian.ply");
    let mut reader = BufReader::new(file_bytes);
    let ply_read = read_buff(&mut reader);
    
    let expected = create_tetrahedron_ply(Encoding::BinaryBigEndian);
    
    assert_eq!(expected.header, ply_read.header);
    assert_eq!(expected.payload, ply_read.payload);
    
    // Round-trip: write and read back
    let written = write_buff(&ply_read);
    let mut reader2 = BufReader::new(&written[..]);
    let ply_roundtrip = read_buff(&mut reader2);
    assert_eq!(ply_read, ply_roundtrip);
}
