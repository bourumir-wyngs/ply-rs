use ply_rs_bw as ply;

/// Sometimes only the metadata is interesting to us.
/// Reading the entire PLY file would be a waste of resources.
fn main() {
    // Set up a reader, in this case a file.
    let path = "example_plys/greg_turk_example1_ok_ascii.ply";
    let f = std::fs::File::open(path).unwrap();

    // Split parsing uses parser::Reader so the parser can keep state such as
    // file-relative line tracking across header and payload parsing.
    let mut reader = ply::parser::Reader::new(std::io::BufReader::new(f));

    // Create a parser.
    let p = ply::parser::Parser::<ply::ply::DefaultElement>::new();

    // Read only the header.
    let header = p.read_header(&mut reader).unwrap();

    // Proof that data has been read.
    println!("Read ply header: {:#?}", header);
}
