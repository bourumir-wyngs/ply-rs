use ply_rs_bw as ply;

/// Demonstrates simplest use case for reading from a file.
fn main() {
    // Set up a reader, in this case a file.
    let path = "example_plys/greg_turk_example1_ok_ascii.ply";
    let mut f = std::fs::File::open(path).unwrap();

    // Create a parser.
    let p = ply::parser::Parser::<ply::ply::DefaultElement>::new();

    // Read the entire file in one step.
    let ply = p.read_ply(&mut f).unwrap();

    // Proof that data has been read.
    println!("Read ply data: {:#?}", ply);
}
