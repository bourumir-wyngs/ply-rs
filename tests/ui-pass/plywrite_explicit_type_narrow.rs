use ply_rs_bw::PlyWrite;

#[derive(PlyWrite)]
struct MyStruct {
    #[ply(type = "short")]
    foo: i32,
    #[ply(type = "float")]
    bar: f64,
}

fn main() {
    let _ = MyStruct { foo: 1, bar: 2.0 };
}
