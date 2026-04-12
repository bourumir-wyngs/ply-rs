use ply_rs_bw::parser;
use ply_rs_bw::ply;

/// We know what data we want to read, so we can parse straight into our own structs.
#[derive(Debug)] // Not necessary for parsing, only for printing at the end of the example.
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug)]
struct Face {
    vertex_index: Vec<i32>,
}

// The structs need to implement PropertyAccess so the parser knows how to write
// parsed properties into them.
impl ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Vertex {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
    fn set_property(&mut self, key: &str, property: ply::Property) {
        match (key, property) {
            ("x", ply::Property::Float(v)) => self.x = v,
            ("y", ply::Property::Float(v)) => self.y = v,
            ("z", ply::Property::Float(v)) => self.z = v,
            (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
        }
    }
}

// Same thing for Face.
impl ply::PropertyAccess for Face {
    fn new() -> Self {
        Face {
            vertex_index: Vec::new(),
        }
    }
    fn set_property(&mut self, key: &str, property: ply::Property) {
        match (key, property) {
            ("vertex_index", ply::Property::ListInt(vec)) => self.vertex_index = vec,
            (k, _) => panic!("Face: Unexpected key/value combination: key: {}", k),
        }
    }
}

/// Demonstrates split parsing into custom element types.
fn main() {
    // Set up a reader, in this case a file.
    let path = "example_plys/greg_turk_example1_ok_ascii.ply";
    let f = std::fs::File::open(path).unwrap();

    // parser::Reader keeps parser state alive across read_header and
    // read_payload_for_element, including file-relative line tracking.
    let mut f = parser::Reader::new(std::io::BufReader::new(f));

    // Create a parser for each struct. Parsers are cheap objects.
    let vertex_parser = parser::Parser::<Vertex>::new();
    let face_parser = parser::Parser::<Face>::new();

    // First consume the header.
    // We also could use `face_parser`; parser configuration is its only state.
    // The reading position only depends on `f`.
    let header = vertex_parser.read_header(&mut f).unwrap();

    // Depending on the header, read the data into our structs.
    let mut vertex_list = Vec::new();
    let mut face_list = Vec::new();
    for (_ignore_key, element) in &header.elements {
        // We could also just parse them in sequence, but the file format might change.
        match element.name.as_ref() {
            "vertex" => {
                vertex_list = vertex_parser
                    .read_payload_for_element(&mut f, element, &header)
                    .unwrap();
            }
            "face" => {
                face_list = face_parser
                    .read_payload_for_element(&mut f, element, &header)
                    .unwrap();
            }
            _ => panic!("Unexpected element!"),
        }
    }

    // Proof that data has been read.
    println!("header: {:#?}", header);
    println!("vertex list: {:#?}", vertex_list);
    println!("face list: {:#?}", face_list);
}
