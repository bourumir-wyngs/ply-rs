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

fn create_all_scalars_ply() -> Ply {
    let mut ply = Ply::new();
    let mut e = ElementDef::new("scalars".to_string());
    
    // Add property definitions
    e.properties.add(PropertyDef::new("c".to_string(), PropertyType::Scalar(ScalarType::Char)));
    e.properties.add(PropertyDef::new("uc".to_string(), PropertyType::Scalar(ScalarType::UChar)));
    e.properties.add(PropertyDef::new("s".to_string(), PropertyType::Scalar(ScalarType::Short)));
    e.properties.add(PropertyDef::new("us".to_string(), PropertyType::Scalar(ScalarType::UShort)));
    e.properties.add(PropertyDef::new("i".to_string(), PropertyType::Scalar(ScalarType::Int)));
    e.properties.add(PropertyDef::new("ui".to_string(), PropertyType::Scalar(ScalarType::UInt)));
    e.properties.add(PropertyDef::new("f".to_string(), PropertyType::Scalar(ScalarType::Float)));
    e.properties.add(PropertyDef::new("d".to_string(), PropertyType::Scalar(ScalarType::Double)));

    ply.header.elements.add(e);

    let mut payload = Vec::new();

    // Helper to add a row
    let add_row = |c: i8, uc: u8, s: i16, us: u16, i: i32, ui: u32, f: f32, d: f64| -> DefaultElement {
         let mut p = KeyMap::new();
         p.insert("c".to_string(), Property::Char(c));
         p.insert("uc".to_string(), Property::UChar(uc));
         p.insert("s".to_string(), Property::Short(s));
         p.insert("us".to_string(), Property::UShort(us));
         p.insert("i".to_string(), Property::Int(i));
         p.insert("ui".to_string(), Property::UInt(ui));
         p.insert("f".to_string(), Property::Float(f));
         p.insert("d".to_string(), Property::Double(d));
         p
    };

    // Element 1: Min values
    payload.push(add_row(i8::MIN, u8::MIN, i16::MIN, u16::MIN, i32::MIN, u32::MIN, f32::MIN, f64::MIN));
    // Element 2: Max values
    payload.push(add_row(i8::MAX, u8::MAX, i16::MAX, u16::MAX, i32::MAX, u32::MAX, f32::MAX, f64::MAX));
    // Element 3: Zero
    payload.push(add_row(0, 0, 0, 0, 0, 0, 0.0, 0.0));
    // Element 4: Mixed
    payload.push(add_row(-12, 200, -1000, 50000, -100000, 3000000, 1.234e-5, -9.876e10));

    ply.payload.insert("scalars".to_string(), payload);
    ply.make_consistent().unwrap();
    ply
}

fn create_all_lists_ply() -> Ply {
    let mut ply = Ply::new();
    let mut e = ElementDef::new("lists".to_string());
    
    // Add property definitions - using UChar as length type for simplicity
    e.properties.add(PropertyDef::new("lc".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::Char)));
    e.properties.add(PropertyDef::new("luc".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::UChar)));
    e.properties.add(PropertyDef::new("ls".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::Short)));
    e.properties.add(PropertyDef::new("lus".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::UShort)));
    e.properties.add(PropertyDef::new("li".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::Int)));
    e.properties.add(PropertyDef::new("lui".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::UInt)));
    e.properties.add(PropertyDef::new("lf".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::Float)));
    e.properties.add(PropertyDef::new("ld".to_string(), PropertyType::List(ScalarType::UChar, ScalarType::Double)));

    ply.header.elements.add(e);

    let mut payload = Vec::new();

    let mut row1 = KeyMap::new();
    row1.insert("lc".to_string(), Property::ListChar(vec![1, -2, 3]));
    row1.insert("luc".to_string(), Property::ListUChar(vec![10, 20, 30]));
    row1.insert("ls".to_string(), Property::ListShort(vec![-100, 200]));
    row1.insert("lus".to_string(), Property::ListUShort(vec![1000, 2000]));
    row1.insert("li".to_string(), Property::ListInt(vec![-10000, 20000]));
    row1.insert("lui".to_string(), Property::ListUInt(vec![10000, 20000]));
    row1.insert("lf".to_string(), Property::ListFloat(vec![1.1, -2.2, 3.3]));
    row1.insert("ld".to_string(), Property::ListDouble(vec![1.1111111, -2.2222222]));
    payload.push(row1);

    // Empty lists
    let mut row2 = KeyMap::new();
    row2.insert("lc".to_string(), Property::ListChar(vec![]));
    row2.insert("luc".to_string(), Property::ListUChar(vec![]));
    row2.insert("ls".to_string(), Property::ListShort(vec![]));
    row2.insert("lus".to_string(), Property::ListUShort(vec![]));
    row2.insert("li".to_string(), Property::ListInt(vec![]));
    row2.insert("lui".to_string(), Property::ListUInt(vec![]));
    row2.insert("lf".to_string(), Property::ListFloat(vec![]));
    row2.insert("ld".to_string(), Property::ListDouble(vec![]));
    payload.push(row2);

    ply.payload.insert("lists".to_string(), payload);
    ply.make_consistent().unwrap();
    ply
}

#[test]
fn write_ascii_all_scalars() {
    let ply = create_all_scalars_ply();
    read_write_ply(&ply);
}

#[test]
fn write_ascii_all_lists() {
    let ply = create_all_lists_ply();
    read_write_ply(&ply);
}
