use ply_rs_bw::ply::{ Ply, DefaultElement };
use ply_rs_bw::writer::{ Writer };

/// Demonstrates simplest use case for writing an empty PLY file.
fn main() {
    // set up a target, could also be a file
    let mut buf = Vec::<u8>::new();

    // create a ply object
    let mut ply = Ply::<DefaultElement>::new();

    // set up a writer
    let w = Writer::new();
    let written = w.write_ply(&mut buf, &mut ply).unwrap();
    println!("{} bytes written", written);
    println!("buffer size: {}", buf.len());

    // proof that data has been read

    // We can use `from_utf8` since PLY files only contain ascii characters
    let output = String::from_utf8(buf).unwrap();
    println!("Written data:\n{}", output);
}
