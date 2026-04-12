use ply_rs_bw::ply::*;
use ply_rs_bw::*;

use std::io::{BufReader, Cursor};

fn try_read_from_bytes(bytes: &[u8]) -> parser::Result<Ply<DefaultElement>> {
    let mut reader = BufReader::new(bytes);
    let p = parser::Parser::<DefaultElement>::new();
    p.read_ply(&mut reader)
}

fn single_property_header(
    encoding: Encoding,
    element_name: &str,
    count: usize,
    property_name: &str,
    data_type: PropertyType,
) -> Header {
    let mut header = Header::new();
    header.encoding = encoding;

    let mut element = ElementDef::new(element_name.to_string());
    element.count = count;
    element
        .properties
        .add(PropertyDef::new(property_name.to_string(), data_type));
    header.elements.add(element);

    header
}

#[test]
fn parser_default_and_read_ply_header_ok() {
    let bytes = b"ply\nformat ascii 1.0\nend_header\n";
    let p = parser::Parser::<DefaultElement>::default();
    let mut cur = Cursor::new(&bytes[..]);
    let header = p.read_ply_header(&mut cur).expect("header should parse");
    assert_eq!(header.encoding, Encoding::Ascii);
}

#[test]
fn parser_read_header_line_invalid_is_error() {
    let p = parser::Parser::<DefaultElement>::new();
    let err = p
        .read_header_line("this is not a ply header line")
        .expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_header_contradicting_format_is_error() {
    let bytes = b"ply\n\
format ascii 1.0\n\
format binary_little_endian 1.0\n\
end_header\n";
    let err = try_read_from_bytes(bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_header_property_without_element_is_error() {
    let bytes = b"ply\n\
format ascii 1.0\n\
property int x\n\
end_header\n";
    let err = try_read_from_bytes(bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_header_missing_format_line_is_error() {
    let bytes = b"ply\ncomment hi\nend_header\n";
    let err = try_read_from_bytes(bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_read_payload_method_ok() {
    let bytes = b"ply\n\
format ascii 1.0\n\
element point 1\n\
property int x\n\
    end_header\n\
7\n";
    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let payload = p
        .read_payload(&mut reader, &header)
        .expect("payload should parse");
    assert_eq!(payload["point"].len(), 1);
}

#[test]
fn parser_payload_only_read_payload_starts_lines_at_one() {
    let header = single_property_header(
        Encoding::Ascii,
        "point",
        1,
        "x",
        PropertyType::Scalar(ScalarType::Int),
    );
    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&b"not-an-int\n"[..]));

    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(err.line(), Some(1));
    assert!(err.to_string().contains("Line 1:"));
}

#[test]
fn parser_payload_only_read_payload_for_element_keeps_later_lines_one_based() {
    let header = single_property_header(
        Encoding::BinaryLittleEndian,
        "point",
        2,
        "x",
        PropertyType::Scalar(ScalarType::Int),
    );
    let element_def = header.elements.get("point").expect("point element");

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7i32.to_le_bytes());

    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let err = p
        .read_payload_for_element(&mut reader, element_def, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert_eq!(err.line(), Some(2));
    assert!(err.to_string().contains("Line 2:"));
}

#[test]
fn parser_split_reader_reports_file_relative_payload_lines() {
    let bytes = b"ply\n\
format ascii 1.0\n\
element point 2\n\
property int x\n\
end_header\n\
7\n";
    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert_eq!(err.line(), Some(7));
    assert!(err.to_string().contains("Line 7:"));
}

#[test]
fn parser_split_reader_reports_file_relative_binary_payload_lines() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element point 2\n\
property int x\n\
end_header\n",
    );
    bytes.extend_from_slice(&7i32.to_le_bytes());

    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert_eq!(err.line(), Some(7));
    assert!(err.to_string().contains("Line 7:"));
}

#[test]
fn parser_split_reader_keeps_line_numbers_across_multiple_elements() {
    let bytes = b"ply\n\
format ascii 1.0\n\
element vertex 1\n\
property float x\n\
element face 1\n\
property list uchar int vertex_index\n\
end_header\n\
1.0\n\
2 0\n";

    let vertex_parser = parser::Parser::<DefaultElement>::new();
    let face_parser = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = vertex_parser
        .read_header(&mut reader)
        .expect("header should parse");
    let vertex_element = header.elements.get("vertex").expect("vertex element");
    let face_element = header.elements.get("face").expect("face element");

    let vertices = vertex_parser
        .read_payload_for_element(&mut reader, vertex_element, &header)
        .expect("vertex payload should parse");
    assert_eq!(vertices.len(), 1);

    let err = face_parser
        .read_payload_for_element(&mut reader, face_element, &header)
        .expect_err("face payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(err.line(), Some(9));
    assert!(err.to_string().contains("Line 9:"));
}

#[test]
fn parser_split_reader_keeps_binary_line_numbers_across_zero_count_elements() {
    let bytes = b"ply\n\
format binary_little_endian 1.0\n\
element vertex 0\n\
property int x\n\
element face 1\n\
property int y\n\
end_header\n";

    let vertex_parser = parser::Parser::<DefaultElement>::new();
    let face_parser = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = vertex_parser
        .read_header(&mut reader)
        .expect("header should parse");
    let vertex_element = header.elements.get("vertex").expect("vertex element");
    let face_element = header.elements.get("face").expect("face element");

    let vertices = vertex_parser
        .read_payload_for_element(&mut reader, vertex_element, &header)
        .expect("vertex payload should parse");
    assert!(vertices.is_empty());

    let err = face_parser
        .read_payload_for_element(&mut reader, face_element, &header)
        .expect_err("face payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert_eq!(err.line(), Some(8));
    assert!(err.to_string().contains("Line 8:"));
}

#[test]
fn parser_split_reader_reports_binary_invalid_data_lines() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element point 1\n\
property float x\n\
end_header\n",
    );
    bytes.extend_from_slice(&1.5f32.to_le_bytes());

    let p = parser::Parser::<RejectFloatElem>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(err.line(), Some(6));
    assert!(err.to_string().contains("Line 6:"));
    assert!(err.to_string().contains("PLY property 'x'"));
}

#[test]
fn parser_split_reader_reports_binary_invalid_input_lines() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element face 1\n\
property list int int vertex_index\n\
end_header\n",
    );
    bytes.extend_from_slice(&(-1i32).to_le_bytes());

    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(err.line(), Some(6));
    assert!(err.to_string().contains("Line 6:"));
    assert!(err.to_string().contains("List length cannot be negative"));
}

#[test]
fn parser_split_reader_preserves_binary_list_eof_kind_and_line() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element face 1\n\
property list uchar int vertex_index\n\
end_header\n",
    );
    bytes.push(2);
    bytes.extend_from_slice(&7i32.to_le_bytes());

    let p = parser::Parser::<DefaultElement>::new();
    let mut reader = parser::Reader::new(BufReader::new(&bytes[..]));
    let header = p.read_header(&mut reader).expect("header should parse");
    let err = p
        .read_payload(&mut reader, &header)
        .expect_err("payload should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert_eq!(err.line(), Some(6));
    assert!(err.to_string().contains("Line 6:"));
    assert!(
        err.to_string()
            .contains("Couldn't find a list element at index 1")
    );
}

#[test]
fn parser_read_little_endian_element_ok() {
    let p = parser::Parser::<DefaultElement>::new();

    let mut e = ElementDef::new("v".to_string());
    e.properties.add(PropertyDef::new(
        "x".to_string(),
        PropertyType::Scalar(ScalarType::Int),
    ));

    let mut cur = Cursor::new(42i32.to_le_bytes());
    let elem = p
        .read_little_endian_element(&mut cur, &e)
        .expect("should read element");
    assert_eq!(elem["x"], Property::Int(42));
}

#[test]
fn parser_read_big_endian_element_ok() {
    let p = parser::Parser::<DefaultElement>::new();

    let mut e = ElementDef::new("v".to_string());
    e.properties.add(PropertyDef::new(
        "x".to_string(),
        PropertyType::Scalar(ScalarType::Int),
    ));

    let mut cur = Cursor::new(42i32.to_be_bytes());
    let elem = p
        .read_big_endian_element(&mut cur, &e)
        .expect("should read element");
    assert_eq!(elem["x"], Property::Int(42));
}

#[test]
fn parser_binary_scalar_char_uchar_short_ushort_ok() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element v 1\n\
property char a\n\
property uchar b\n\
property short c\n\
property ushort d\n\
end_header\n",
    );
    bytes.push(0x80u8); // i8 = -128
    bytes.push(0xFFu8); // u8 = 255
    bytes.extend_from_slice(&(-12345i16).to_le_bytes());
    bytes.extend_from_slice(&(54321u16).to_le_bytes());

    let ply = try_read_from_bytes(&bytes).expect("should parse");
    let v = &ply.payload["v"][0];
    assert_eq!(v["a"], Property::Char(-128));
    assert_eq!(v["b"], Property::UChar(255));
    assert_eq!(v["c"], Property::Short(-12345));
    assert_eq!(v["d"], Property::UShort(54321));
}

#[test]
fn parser_binary_negative_list_len_i8_is_error() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element face 1\n\
property list char uchar idx\n\
end_header\n",
    );
    bytes.push(0xFFu8); // i8 = -1
    let err = try_read_from_bytes(&bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_binary_negative_list_len_i16_is_error() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"ply\n\
format binary_little_endian 1.0\n\
element face 1\n\
property list short uchar idx\n\
end_header\n",
    );
    bytes.extend_from_slice(&(-1i16).to_le_bytes());
    let err = try_read_from_bytes(&bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_binary_list_index_float_is_error() {
    // The list index type is invalid (float), so the parser should reject it.
    // No payload bytes are needed to reach this branch.
    let bytes = b"ply\n\
format binary_little_endian 1.0\n\
element face 1\n\
property list float int idx\n\
end_header\n";
    let err = try_read_from_bytes(bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parser_binary_payload_unexpected_eof_is_error() {
    // Declares one i32 but provides no payload bytes.
    let bytes = b"ply\n\
format binary_little_endian 1.0\n\
element v 1\n\
property int x\n\
end_header\n";
    let err = try_read_from_bytes(bytes).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}

#[test]
fn writer_default_write_ply_inconsistent_is_error() {
    let mut ply = Ply::<DefaultElement>::new();
    ply.header.comments.push("bad\ncomment".to_string());

    let w = writer::Writer::<DefaultElement>::default();
    let err = w
        .write_ply(&mut Vec::<u8>::new(), &mut ply)
        .expect_err("should fail consistency check");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn writer_unchecked_rejects_list_index_float_in_header() {
    // `write_ply_unchecked` still validates header property types while serializing the header.
    let mut ply = Ply::<DefaultElement>::new();
    ply.header.encoding = Encoding::BinaryLittleEndian;

    let mut e = ElementDef::new("face".to_string());
    e.properties.add(PropertyDef::new(
        "idx".to_string(),
        PropertyType::List(ScalarType::Float, ScalarType::Int),
    ));
    ply.header.elements.add(e);
    ply.payload.insert("face".to_string(), vec![KeyMap::new()]);
    // Ensure counts are set etc.
    ply.make_consistent().unwrap();

    let w = writer::Writer::<DefaultElement>::new();
    let err = w
        .write_ply_unchecked(&mut Vec::<u8>::new(), &ply)
        .expect_err("should fail on invalid list index type");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[derive(Debug)]
struct BinElem {
    c: i8,
    uc: u8,
    s: i16,
    us: u16,
    d: f64,
    li: Vec<i32>,
}

impl PropertyAccess for BinElem {
    fn new() -> Self {
        Self {
            c: 0,
            uc: 0,
            s: 0,
            us: 0,
            d: 0.0,
            li: Vec::new(),
        }
    }

    fn get_char(&self, property_name: &str) -> Option<i8> {
        (property_name == "c").then_some(self.c)
    }
    fn get_uchar(&self, property_name: &str) -> Option<u8> {
        (property_name == "uc").then_some(self.uc)
    }
    fn get_short(&self, property_name: &str) -> Option<i16> {
        (property_name == "s").then_some(self.s)
    }
    fn get_ushort(&self, property_name: &str) -> Option<u16> {
        (property_name == "us").then_some(self.us)
    }
    fn get_double(&self, property_name: &str) -> Option<f64> {
        (property_name == "d").then_some(self.d)
    }
    fn get_list_int(&self, property_name: &str) -> Option<&[i32]> {
        (property_name == "li").then_some(self.li.as_slice())
    }
}

#[test]
fn writer_binary_element_writes_multiple_scalar_and_list_variants() {
    let e = BinElem {
        c: -128,
        uc: 255,
        s: -12345,
        us: 54321,
        d: 1.25,
        li: vec![1, -2, 3],
    };

    let mut def = ElementDef::new("v".to_string());
    def.properties.add(PropertyDef::new(
        "c".to_string(),
        PropertyType::Scalar(ScalarType::Char),
    ));
    def.properties.add(PropertyDef::new(
        "uc".to_string(),
        PropertyType::Scalar(ScalarType::UChar),
    ));
    def.properties.add(PropertyDef::new(
        "s".to_string(),
        PropertyType::Scalar(ScalarType::Short),
    ));
    def.properties.add(PropertyDef::new(
        "us".to_string(),
        PropertyType::Scalar(ScalarType::UShort),
    ));
    def.properties.add(PropertyDef::new(
        "d".to_string(),
        PropertyType::Scalar(ScalarType::Double),
    ));
    def.properties.add(PropertyDef::new(
        "li".to_string(),
        PropertyType::List(ScalarType::UChar, ScalarType::Int),
    ));

    let w = writer::Writer::<BinElem>::new();
    let mut out = Vec::<u8>::new();
    let written = w
        .write_little_endian_element(&mut out, &e, &def)
        .expect("should write");

    // 1 (i8) + 1 (u8) + 2 (i16) + 2 (u16) + 8 (f64) + 1 (u8 len) + 3*4 (i32 list)
    assert_eq!(written, 1 + 1 + 2 + 2 + 8 + 1 + 12);
    assert_eq!(out.len(), written);
}

#[derive(Debug)]
struct DirectParseElem {
    x: f32,
    labels: Vec<i32>,
}

impl PropertyAccess for DirectParseElem {
    fn new() -> Self {
        Self {
            x: 0.0,
            labels: vec![-1, -1],
        }
    }

    fn set_property(&mut self, property_name: &str, _property: Property) -> PropertyAccessResult {
        panic!("optimized parser should not call set_property for '{property_name}'");
    }

    fn set_float(&mut self, property_name: &str, value: f32) -> PropertyAccessResult {
        match property_name {
            "x" => {
                self.x = value;
                PropertyAccessResult::Set
            }
            other => panic!("unexpected float property: {other}"),
        }
    }

    fn begin_list_int(&mut self, property_name: &str, _len: usize) -> BeginList<'_, i32> {
        if property_name == "labels" {
            BeginList::Fill(&mut self.labels)
        } else {
            BeginList::UseSetter
        }
    }
}

fn optimized_elem_def() -> ElementDef {
    let mut def = ElementDef::new("vertex".to_string());
    def.properties.add(PropertyDef::new(
        "x".to_string(),
        PropertyType::Scalar(ScalarType::Float),
    ));
    def.properties.add(PropertyDef::new(
        "labels".to_string(),
        PropertyType::List(ScalarType::UChar, ScalarType::Int),
    ));
    def
}

#[test]
fn parser_ascii_custom_hooks_bypass_set_property() {
    let p = parser::Parser::<DirectParseElem>::new();
    let elem = p
        .read_ascii_element("1.5 3 1 2 3", &optimized_elem_def())
        .expect("should parse");

    assert_eq!(elem.x, 1.5);
    assert_eq!(elem.labels, vec![1, 2, 3]);
}

#[test]
fn parser_binary_custom_hooks_fill_final_list_storage() {
    let p = parser::Parser::<DirectParseElem>::new();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1.5f32.to_le_bytes());
    bytes.push(3);
    bytes.extend_from_slice(&1i32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&3i32.to_le_bytes());

    let mut cur = Cursor::new(bytes);
    let elem = p
        .read_little_endian_element(&mut cur, &optimized_elem_def())
        .expect("should parse");

    assert_eq!(elem.x, 1.5);
    assert_eq!(elem.labels, vec![1, 2, 3]);
}

#[derive(Debug)]
struct RejectFloatElem;

impl PropertyAccess for RejectFloatElem {
    fn new() -> Self {
        Self
    }

    fn set_float(&mut self, property_name: &str, _value: f32) -> PropertyAccessResult {
        match property_name {
            "x" => PropertyAccessResult::UnsupportedType,
            other => panic!("unexpected float property: {other}"),
        }
    }
}

#[test]
fn parser_reports_unsupported_destination_property_types() {
    let p = parser::Parser::<RejectFloatElem>::new();
    let mut def = ElementDef::new("vertex".to_string());
    def.properties.add(PropertyDef::new(
        "x".to_string(),
        PropertyType::Scalar(ScalarType::Float),
    ));

    let err = p.read_ascii_element("1.5", &def).expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("PLY property 'x'"));
}

#[derive(Debug)]
struct RejectListIntBeginElem;

impl PropertyAccess for RejectListIntBeginElem {
    fn new() -> Self {
        Self
    }

    fn begin_list_int(&mut self, property_name: &str, _len: usize) -> BeginList<'_, i32> {
        match property_name {
            "labels" => BeginList::UnsupportedType,
            other => panic!("unexpected list property: {other}"),
        }
    }
}

#[test]
fn parser_reports_unsupported_list_via_begin_list_ascii() {
    let p = parser::Parser::<RejectListIntBeginElem>::new();
    let err = p
        .read_ascii_element("1.5 3 1 2 3", &optimized_elem_def())
        .expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("PLY property 'labels'"));
}

#[test]
fn parser_reports_unsupported_list_via_begin_list_binary() {
    let p = parser::Parser::<RejectListIntBeginElem>::new();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1.5f32.to_le_bytes());
    bytes.push(3);

    let mut cur = Cursor::new(bytes);
    let err = p
        .read_little_endian_element(&mut cur, &optimized_elem_def())
        .expect_err("should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("PLY property 'labels'"));
}
