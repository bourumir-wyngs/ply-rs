#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut cursor = std::io::Cursor::new(data);
    let p = ply_rs_bw::parser::Parser::<ply_rs_bw::ply::DefaultElement>::new();
    let _ = p.read_ply_header(&mut cursor);
});
