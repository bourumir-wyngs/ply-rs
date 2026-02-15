use ply_rs_bw::{PlyRead, PlyWrite, ToPly, FromPly};
use ply_rs_bw::ply::{Property, PropertyAccess};
use std::borrow::Cow;

#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct AllScalars {
    #[ply(name = "char")]
    c: i8,
    #[ply(name = "uchar")]
    uc: u8,
    #[ply(name = "short")]
    s: i16,
    #[ply(name = "ushort")]
    us: u16,
    #[ply(name = "int")]
    i: i32,
    #[ply(name = "uint")]
    ui: u32,
    #[ply(name = "float")]
    f: f32,
    #[ply(name = "double")]
    d: f64,
}

#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct AllLists {
    #[ply(name = "list_char")]
    lc: Vec<i8>,
    #[ply(name = "list_uchar")]
    luc: Vec<u8>,
    #[ply(name = "list_short")]
    ls: Vec<i16>,
    #[ply(name = "list_ushort")]
    lus: Vec<u16>,
    #[ply(name = "list_int")]
    li: Vec<i32>,
    #[ply(name = "list_uint")]
    lui: Vec<u32>,
    #[ply(name = "list_float")]
    lf: Vec<f32>,
    #[ply(name = "list_double")]
    ld: Vec<f64>,
}

#[test]
fn test_all_scalars_set_property() {
    let mut s = AllScalars::default();
    s.set_property("char", Property::Char(1));
    s.set_property("uchar", Property::UChar(2));
    s.set_property("short", Property::Short(3));
    s.set_property("ushort", Property::UShort(4));
    s.set_property("int", Property::Int(5));
    s.set_property("uint", Property::UInt(6));
    s.set_property("float", Property::Float(7.0));
    s.set_property("double", Property::Double(8.0));

    assert_eq!(s.c, 1);
    assert_eq!(s.uc, 2);
    assert_eq!(s.s, 3);
    assert_eq!(s.us, 4);
    assert_eq!(s.i, 5);
    assert_eq!(s.ui, 6);
    assert_eq!(s.f, 7.0);
    assert_eq!(s.d, 8.0);
}

#[test]
fn test_all_scalars_conversion() {
    let mut s = AllScalars::default();
    // Test that double can be set to f32 field
    s.set_property("float", Property::Double(7.5));
    assert_eq!(s.f, 7.5);

    // Test that int can be set to f32 field
    s.set_property("float", Property::Int(42));
    assert_eq!(s.f, 42.0);

    // Test that float can be set to i32 field
    s.set_property("int", Property::Float(10.9));
    assert_eq!(s.i, 10); // cast as i32
}

#[test]
fn test_all_lists_set_property() {
    let mut l = AllLists::default();
    l.set_property("list_char", Property::ListChar(vec![1]));
    l.set_property("list_uchar", Property::ListUChar(vec![2]));
    l.set_property("list_short", Property::ListShort(vec![3]));
    l.set_property("list_ushort", Property::ListUShort(vec![4]));
    l.set_property("list_int", Property::ListInt(vec![5]));
    l.set_property("list_uint", Property::ListUInt(vec![6]));
    l.set_property("list_float", Property::ListFloat(vec![7.0]));
    l.set_property("list_double", Property::ListDouble(vec![8.0]));

    assert_eq!(l.lc, vec![1]);
    assert_eq!(l.luc, vec![2]);
    assert_eq!(l.ls, vec![3]);
    assert_eq!(l.lus, vec![4]);
    assert_eq!(l.li, vec![5]);
    assert_eq!(l.lui, vec![6]);
    assert_eq!(l.lf, vec![7.0]);
    assert_eq!(l.ld, vec![8.0]);
}

#[test]
fn test_all_lists_conversion() {
    let mut l = AllLists::default();
    // Test conversion between list types
    l.set_property("list_float", Property::ListDouble(vec![1.5, 2.5]));
    assert_eq!(l.lf, vec![1.5, 2.5]);

    l.set_property("list_int", Property::ListFloat(vec![1.1, 2.9]));
    assert_eq!(l.li, vec![1, 2]);
}

#[derive(Debug, Default, PlyRead, Clone, PartialEq)]
struct OptionalFields {
    #[ply(name = "x")]
    x: f32,
    #[ply(name = "y")]
    y: Option<f32>,
}

#[test]
fn test_optional_fields() {
    let mut o = OptionalFields::default();
    assert_eq!(o.x, 0.0);
    assert_eq!(o.y, None);

    o.set_property("x", Property::Float(1.0));
    o.set_property("y", Property::Float(2.0));
    assert_eq!(o.x, 1.0);
    assert_eq!(o.y, Some(2.0));

    // If property is not set, it should remain None (or its previous value)
    let mut o2 = OptionalFields::default();
    o2.set_property("x", Property::Float(1.0));
    // "y" is missing
    assert_eq!(o2.x, 1.0);
    assert_eq!(o2.y, None);
}

#[test]
fn test_getters() {
    let s = AllScalars {
        c: 1, uc: 2, s: 3, us: 4, i: 5, ui: 6, f: 7.0, d: 8.0,
    };
    assert_eq!(s.get_char("char"), Some(1));
    assert_eq!(s.get_uchar("uchar"), Some(2));
    assert_eq!(s.get_short("short"), Some(3));
    assert_eq!(s.get_ushort("ushort"), Some(4));
    assert_eq!(s.get_int("int"), Some(5));
    assert_eq!(s.get_uint("uint"), Some(6));
    assert_eq!(s.get_float("float"), Some(7.0));
    assert_eq!(s.get_double("double"), Some(8.0));
    assert_eq!(s.get_float("non_existent"), None);
}

#[test]
fn test_list_getters() {
    let l = AllLists {
        lc: vec![1], luc: vec![2], ls: vec![3], lus: vec![4],
        li: vec![5], lui: vec![6], lf: vec![7.0], ld: vec![8.0],
    };
    assert_eq!(l.get_list_char("list_char"), Some(Cow::Borrowed([1i8].as_slice())));
    assert_eq!(l.get_list_uchar("list_uchar"), Some(Cow::Borrowed([2u8].as_slice())));
    assert_eq!(l.get_list_short("list_short"), Some(Cow::Borrowed([3i16].as_slice())));
    assert_eq!(l.get_list_ushort("list_ushort"), Some(Cow::Borrowed([4u16].as_slice())));
    assert_eq!(l.get_list_int("list_int"), Some(Cow::Borrowed([5i32].as_slice())));
    assert_eq!(l.get_list_uint("list_uint"), Some(Cow::Borrowed([6u32].as_slice())));
    assert_eq!(l.get_list_float("list_float"), Some(Cow::Borrowed([7.0f32].as_slice())));
    assert_eq!(l.get_list_double("list_double"), Some(Cow::Borrowed([8.0f64].as_slice())));
    assert_eq!(l.get_list_float("non_existent"), None);
}

#[derive(Debug, ToPly, FromPly, PartialEq)]
struct SimpleMesh {
    #[ply(name = "v")]
    vertices: Vec<AllScalars>,
}

#[derive(Debug, Default, PlyRead, PlyWrite, Clone, PartialEq)]
struct GenericVertex<T>
where
    T: Default + Copy + 'static + ply_rs_bw::ply::SetProperty<f32> + ply_rs_bw::ply::GetProperty<f32>,
{
    #[ply(type = "float")]
    x: T,
    #[ply(type = "float")]
    y: T,
    #[ply(type = "float")]
    z: T,
}

#[test]
fn test_generic_struct_ply_access() {
    let mut v = GenericVertex::<f32>::default();
    v.set_property("x", Property::Float(1.0));
    v.set_property("y", Property::Float(2.0));
    v.set_property("z", Property::Float(3.0));

    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    assert_eq!(v.z, 3.0);

    // Getters
    // Note: get_float expects f32, so we can only use it if T is f32.
    // Our macro generates match arms that check the field type at compile time.
}

#[derive(Debug, ToPly, FromPly, PartialEq)]
struct GenericMesh<T>
where
    T: Default + Copy + 'static + ply_rs_bw::ply::SetProperty<f32> + ply_rs_bw::ply::GetProperty<f32>,
{
    #[ply(name = "vertex")]
    vertices: Vec<GenericVertex<T>>,
}

#[test]
fn test_generic_mesh_roundtrip() {
    let mesh = GenericMesh::<f32> {
        vertices: vec![
            GenericVertex { x: 1.0, y: 2.0, z: 3.0 },
            GenericVertex { x: 4.0, y: 5.0, z: 6.0 },
        ],
    };

    let mut buf = Vec::new();
    mesh.write_ply(&mut buf).unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let read_mesh = GenericMesh::<f32>::read_ply(&mut cursor).unwrap();

    assert_eq!(mesh, read_mesh);
}

#[test]
fn test_simple_mesh_roundtrip() {
    let mesh = SimpleMesh {
        vertices: vec![
            AllScalars { c: 1, uc: 2, s: 3, us: 4, i: 5, ui: 6, f: 7.0, d: 8.0 },
            AllScalars { c: -1, uc: 255, s: -300, us: 4000, i: -500000, ui: 600000, f: -7.1, d: 8.2 },
        ],
    };

    let mut buf = Vec::new();
    mesh.write_ply(&mut buf).unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let read_mesh = SimpleMesh::read_ply(&mut cursor).unwrap();

    assert_eq!(mesh, read_mesh);
}

#[derive(Debug, Default, PlyRead, Clone, PartialEq)]
struct ListWithExplicitTypePA {
    #[ply(name = "indices", type = "uint")]
    indices: Vec<u32>,
}

#[test]
fn test_list_with_explicit_type_pa() {
    let mut l = ListWithExplicitTypePA::default();
    l.set_property("indices", Property::ListInt(vec![1, 2, 3]));
    assert_eq!(l.indices, vec![1, 2, 3]);
}
