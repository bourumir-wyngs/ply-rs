use ply_rs_bw::*;
use std::io::BufReader;

fn read_from_bytes(bytes: &[u8]) -> ply::Ply<ply::DefaultElement> {
    let mut reader = BufReader::new(bytes);
    let p = parser::Parser::<ply::DefaultElement>::new();
    let ply = p.read_ply(&mut reader);
    assert!(ply.is_ok(), "{}", format!("failed: {}", ply.err().unwrap()));
    ply.unwrap()
}

#[test]
fn read_header_crlf_and_empty_comment_objinfo() {
    let txt = b"ply\r\n\
format ascii 1.0\r\n\
comment\r\n\
obj_info\r\n\
end_header\r\n";
    let mut reader = BufReader::new(&txt[..]);
    let p = parser::Parser::<ply::DefaultElement>::new();
    let header = p.read_header(&mut reader).expect("header should parse");
    assert_eq!(header.encoding, ply::Encoding::Ascii);
    // empty comment and obj_info should be captured
    assert_eq!(header.comments.len(), 1);
    assert_eq!(header.comments[0], "");
    assert_eq!(header.obj_infos.len(), 1);
    assert_eq!(header.obj_infos[0], "");
}

#[test]
fn read_ascii_payload_with_whitespace_variations() {
    // Tabs, multiple spaces, leading/trailing spaces, CRLF line endings
    let txt = b"ply\r\n\
format ascii 1.0\r\n\
 element point 3 \r\n\
 property int x\r\n\
 property uint y\r\n\
end_header\r\n\
  -7\t   5  \r\n\
2   \t4\r\n\
   0   0   \r\n";
    let ply = read_from_bytes(txt);
    assert_eq!(ply.header.elements["point"].count, 3);
    let payload = &ply.payload["point"];
    assert_eq!(payload.len(), 3);
    // (-7,5), (2,4), (0,0)
    let get = |i: usize, key: &str| match payload[i][key] {
        ply::Property::Int(v) => v as i64,
        ply::Property::UInt(v) => v as i64,
        _ => panic!("unexpected type"),
    };
    assert_eq!(get(0, "x"), -7);
    assert_eq!(get(0, "y"), 5);
    assert_eq!(get(1, "x"), 2);
    assert_eq!(get(1, "y"), 4);
    assert_eq!(get(2, "x"), 0);
    assert_eq!(get(2, "y"), 0);
}

#[test]
fn read_ascii_no_trailing_newline_in_payload() {
    let txt = b"ply\nformat ascii 1.0\n\
 element value 1\n\
 property float x\n\
end_header\n6.28318530718"; // no trailing newline
    let ply = read_from_bytes(txt);
    assert_eq!(ply.payload["value"].len(), 1);
}

#[test]
fn read_diverse_field_formats() {
    use ply_rs_bw::ply::{PropertyType, ScalarType};
    let cases = vec![
        ("example_plys/diverse_field_formats/floats_ints.ply", ScalarType::Float, ScalarType::Int),
        ("example_plys/diverse_field_formats/floats_shorts.ply", ScalarType::Float, ScalarType::Short),
        ("example_plys/diverse_field_formats/doubles_ints.ply", ScalarType::Double, ScalarType::Int),
        ("example_plys/diverse_field_formats/doubles_shorts.ply", ScalarType::Double, ScalarType::Short),
    ];

    for (path, coord_ty, idx_ty) in cases {
        let mut f = std::fs::File::open(path).expect("fixture should exist");
        let p = parser::Parser::<ply::DefaultElement>::new();
        let ply = p.read_ply(&mut f).expect("should parse");
        assert_eq!(ply.header.encoding, ply::Encoding::BinaryLittleEndian);
        assert_eq!(ply.header.elements["vertex"].count, 3);
        assert_eq!(ply.header.elements["face"].count, 1);
        // Check vertex property types x,y,z
        let vprops = &ply.header.elements["vertex"].properties;
        for key in ["x", "y", "z"].iter() {
            match vprops[*key].data_type {
                PropertyType::Scalar(ref s) => assert_eq!(*s, coord_ty),
                _ => panic!("vertex {:?} should be scalar", key),
            }
        }
        // Check face list type
        let fprops = &ply.header.elements["face"].properties;
        match fprops["vertex_indices"].data_type {
            PropertyType::List(ref len_ty, ref item_ty) => {
                assert_eq!(*len_ty, ScalarType::UChar);
                assert_eq!(*item_ty, idx_ty);
            }
            _ => panic!("vertex_indices should be a list"),
        }
        // basic payload sanity
        assert_eq!(ply.payload["vertex"].len(), 3);
        assert_eq!(ply.payload["face"].len(), 1);
        // vertex_indices list length should be 3
        match ply.payload["face"][0]["vertex_indices"] {
            ply::Property::ListInt(ref v) => assert_eq!(v.len(), 3),
            ply::Property::ListShort(ref v) => assert_eq!(v.len(), 3),
            _ => panic!("unexpected list type for vertex_indices"),
        }
    }
}

#[test]
fn read_header_with_very_long_obj_info() {
    let long = "x".repeat(10_000);
    let txt = format!(
        "ply\nformat ascii 1.0\nobj_info {}\nend_header\n",
        long
    );
    let ply = read_from_bytes(txt.as_bytes());
    assert_eq!(ply.header.obj_infos.len(), 1);
    assert_eq!(ply.header.obj_infos[0].len(), 10_000);
}
