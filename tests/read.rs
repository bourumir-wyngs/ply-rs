use ply_rs_bw::*;
use std::io::BufReader;

type Ply = ply::Ply<ply::DefaultElement>;

// Use include_bytes! to embed files at compile time, avoiding filesystem access.
// This is done for Miri compatibility, as Miri's isolation mode doesn't support file I/O.
fn read_from_bytes(bytes: &[u8]) -> Ply {
    let mut reader = BufReader::new(bytes);
    let p = parser::Parser::new();
    let ply = p.read_ply(&mut reader);
    assert!(ply.is_ok(), "{}", format!("failed: {}", ply.err().unwrap()));
    ply.unwrap()
}

// Embedded PLY files for Miri compatibility
const EMPTY_OK_ASCII: &[u8] = include_bytes!("../example_plys/empty_ok_ascii.ply");
const EMPTY_2_OK_ASCII: &[u8] = include_bytes!("../example_plys/empty_2_ok_ascii.ply");
const EMPTY_2_OK_LITTLE_ENDIAN: &[u8] = include_bytes!("../example_plys/empty_2_ok_little_endian.ply");
const HOUSE_OK_ASCII: &[u8] = include_bytes!("../example_plys/house_ok_ascii.ply");
const HOUSE_2_OK_ASCII: &[u8] = include_bytes!("../example_plys/house_2_ok_ascii.ply");
const HOUSE_2_OK_LITTLE_ENDIAN: &[u8] = include_bytes!("../example_plys/house_2_ok_little_endian.ply");
const GREG_TURK_EXAMPLE1_OK_ASCII: &[u8] = include_bytes!("../example_plys/greg_turk_example1_ok_ascii.ply");
const GREG_TURK_EXAMPLE2_OK_ASCII: &[u8] = include_bytes!("../example_plys/greg_turk_example2_ok_ascii.ply");
const EXPONENT_VALUES_OK_ASCII: &[u8] = include_bytes!("../example_plys/exponent_values_ok_ascii.ply");
const LEADING_SPACES_OK_ASCII: &[u8] = include_bytes!("../example_plys/leading_spaces_ok_ascii.ply");
const ALL_ATOMIC_TYPES_OK_ASCII: &[u8] = include_bytes!("../example_plys/all_atomic_types_ok_ascii.ply");

#[test]
fn read_empty() {
    let ply = read_from_bytes(EMPTY_OK_ASCII);
    assert_eq!(ply.header.elements["face"].count, 0);
    assert!(ply.payload["vertex"].is_empty());
    assert!(ply.payload["face"].is_empty());
}
#[test]
fn read_empy_equal() {
    let bin = read_from_bytes(EMPTY_2_OK_LITTLE_ENDIAN);
    let ascii = read_from_bytes(EMPTY_2_OK_ASCII);
    assert_eq!(bin.header.elements, ascii.header.elements);
    assert_eq!(bin.payload, ascii.payload);
}
#[test]
fn read_house() {
    let ply = read_from_bytes(HOUSE_OK_ASCII);
    println!("Created ply: {:?}", ply);
    assert_eq!(ply.header.elements["face"].count, 3);
    assert_eq!(ply.payload["vertex"].len(), 5);
    assert_eq!(ply.payload["face"].len(), 3);
}
#[test]
fn read_house_equal() {
    let bin = read_from_bytes(HOUSE_2_OK_LITTLE_ENDIAN);
    let ascii = read_from_bytes(HOUSE_2_OK_ASCII);
    assert_eq!(bin.header.elements, ascii.header.elements);
    assert_eq!(bin.payload, ascii.payload);
}
#[test]
fn read_greg_turk_1() {
    let ply = read_from_bytes(GREG_TURK_EXAMPLE1_OK_ASCII);
    println!("Created ply: {:?}", ply);
}
#[test]
fn read_greg_turk_2() {
    let ply = read_from_bytes(GREG_TURK_EXAMPLE2_OK_ASCII);
    println!("Created ply: {:?}", ply);
}
#[test]
fn read_exponent_values_ok() {
    let ply = read_from_bytes(EXPONENT_VALUES_OK_ASCII);
    println!("Created ply: {:?}", ply);
}
#[test]
fn read_leading_spaces_ok() {
    let ply = read_from_bytes(LEADING_SPACES_OK_ASCII);
    println!("Created ply: {:?}", ply);
}
#[test]
fn read_all_atomic_types_ok() {
    let ply = read_from_bytes(ALL_ATOMIC_TYPES_OK_ASCII);
    println!("Created ply: {:?}", ply);
}

mod struct_test_1 {
    use super::ply;
    use super::parser::Parser;
    use super::{read_from_bytes, GREG_TURK_EXAMPLE1_OK_ASCII};
    #[derive(Debug)]
    struct Vertex {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Debug)]
    struct Face {
        vertex_index: Vec<i32>,
    }


    impl ply::PropertyAccess for Vertex {
        fn new() -> Self {
            Vertex {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        }
        fn set_property(&mut self, key: &String, property: ply::Property) {
            match (key.as_str(), property) {
                ("x", ply::Property::Float(v)) => self.x = v,
                ("y", ply::Property::Float(v)) => self.y = v,
                ("z", ply::Property::Float(v)) => self.z = v,
                (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
            }
        }
    }

    // same thing for Face
    impl ply::PropertyAccess for Face {
        fn new() -> Self {
            Face {
                vertex_index: Vec::new(),
            }
        }
        fn set_property(&mut self, key: &String, property: ply::Property) {
            match (key.as_str(), property) {
                ("vertex_index", ply::Property::ListInt(vec)) => self.vertex_index = vec,
                (k, _) => panic!("Face: Unexpected key/value combination: key: {}", k),
            }
        }
    }

    /// Demonstrates simplest use case for reading from a file.
    #[test]
    fn read_into_struct() {
        // Use include_bytes! data for Miri compatibility
        let mut f = std::io::BufReader::new(GREG_TURK_EXAMPLE1_OK_ASCII);

        // Create a parser for each struct. Parsers are cheap objects.
        let vertex_parser = Parser::<Vertex>::new();
        let face_parser = Parser::<Face>::new();

        // lets first consume the header
        // We also could use `face_parser`, The configuration is a parser's only state.
        // The reading position only depends on `f`.
        let header = vertex_parser.read_header(&mut f).unwrap();

        // Depending on the header, read the data into our structs..
        let mut vertex_list = Vec::new();
        let mut face_list = Vec::new();
        for (_ignore_key, element) in &header.elements {
            // we could also just parse them in sequence, but the file format might change
            match element.name.as_ref() {
                "vertex" => {vertex_list = vertex_parser.read_payload_for_element(&mut f, element, &header).unwrap();},
                "face" => {face_list = face_parser.read_payload_for_element(&mut f, element, &header).unwrap();},
                _ => panic!("Unexpected element!"),
            }
        }

        println!("header: {:#?}", header);
        println!("vertex list: {:#?}", vertex_list);
        println!("face list: {:#?}", face_list);

        let ply = read_from_bytes(GREG_TURK_EXAMPLE1_OK_ASCII);

        for (i, vertex) in vertex_list.iter().enumerate() {
            let x = match ply.payload["vertex"][i]["x"] {
                ply::Property::Float(v) => v,
                _ => panic!("Unexpected property."),
            };
            assert_eq!(vertex.x, x);
            let y = match ply.payload["vertex"][i]["y"] {
                ply::Property::Float(v) => v,
                _ => panic!("Unexpected property."),
            };
            assert_eq!(vertex.y, y);
            let z = match ply.payload["vertex"][i]["z"] {
                ply::Property::Float(v) => v,
                _ => panic!("Unexpected property."),
            };
            assert_eq!(vertex.z, z);
        }

        for (i, face) in face_list.iter().enumerate() {
            let v = match ply.payload["face"][i]["vertex_index"] {
                ply::Property::ListInt(ref v) => v,
                _ => panic!("Unexpected property."),
            };
            assert_eq!(face.vertex_index, *v);
        }
    }
}
