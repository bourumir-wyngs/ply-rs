use ply_rs_bw::{PlyRead, PlyWrite};
use ply_rs_bw::ply::{PropertyAccess, WriteSchema, Addable};
use std::borrow::Cow;

#[derive(PlyRead, PlyWrite, Default, Debug, Clone)]
struct Face {
    #[ply(name = "vertex_indices", type = "uint")]
    indices: Vec<u64>,
}

#[test]
fn test_face_indices_list_uint_access() {
    let face = Face {
        indices: vec![1, 2, 3, 100],
    };

    // Verify list_uint access
    // `u64` should be converted to `u32` (uint)
    // The previous bug was that this returned None because the macro skipped generation.
    let result = face.get_list_uint("vertex_indices");
    
    assert!(result.is_some(), "Expected list_uint getter to be generated and return Some, got None");
    
    let cow = result.unwrap();
    assert!(matches!(cow, Cow::Owned(_)), "Expected Cow::Owned due to conversion (Vec<u64> -> Vec<u32>)");
    
    let expected: Vec<u32> = vec![1, 2, 3, 100];
    assert_eq!(cow.into_owned(), expected);
}

#[test]
fn test_write_face_indices() {
    // Verify that writing actually works using the getter
    let face = Face {
        indices: vec![10, 20, 30],
    };
    
    let writer = ply_rs_bw::writer::Writer::new();
    let mut buf = Vec::new();
    
    // Manually construct the element definition based on WriteSchema
    let mut element_def = ply_rs_bw::ply::ElementDef::new("face".to_string());
    let props = Face::property_type_schema();
    for (name, type_) in props {
        element_def.properties.add(ply_rs_bw::ply::PropertyDef::new(name, type_));
    }
    
    // Use write_ascii_element directly to test writing a single element
    let res = writer.write_ascii_element(&mut buf, &face, &element_def);
    assert!(res.is_ok(), "Failed to write element: {:?}", res.err());
    
    let written = String::from_utf8(buf).unwrap();
    // Format: count index0 index1 index2 ...
    // ASCII list format: "count val val val"
    // So "3 10 20 30" plus newline
    assert!(written.trim().ends_with("3 10 20 30"), "Written content mismatch: {}", written);
}
