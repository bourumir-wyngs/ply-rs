# Ply-rs
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Fuzz & Audit](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml)
[![Miri](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml)
[![crates.io](https://img.shields.io/crates/v/ply-rs_bw.svg)](https://crates.io/crates/ply-rs-bw)
[![MSRV](https://img.shields.io/badge/MSRV-1.87+-lightgray.svg)](./Cargo.toml)
[![API 4.x compatibility](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/ply-rs/rust.yml)](https://github.com/bourumir-wyngs/ply-rs/actions)
[![crates.io](https://img.shields.io/crates/l/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![crates.io](https://img.shields.io/crates/d/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![docs.rs](https://docs.rs/ply-rs-bw/badge.svg)](https://docs.rs/ply-rs-bw)
[![Dependency Vulnerabilities](https://img.shields.io/endpoint?url=https%3A%2F%2Fapi-hooks.soos.io%2Fapi%2Fshieldsio-badges%3FbadgeType%3DDependencyVulnerabilities%26pid%3D3cwrfhxz2%26)](https://app.soos.io)

This is a forked version of the [ply-rs](https://github.com/Fluci/ply-rs) project that was created to address the use of `linked-hash-map` to resolve [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573). 

The crate has been renamed to `ply-rs-bw`, and minor issues were resolved while moving to the Rust 2024 edition.
**MSRV is Rust 1.87**, set intentionally via `rust-version` in `Cargo.toml` so the minimum toolchain matches the edition and APIs this crate is built and tested against.
Additionally, an example has been added to demonstrate how to read PLY files with diverse field types
(e.g., `f32` vs `f64`, `u32` vs `i32`, etc.). Semantic versioning is now adopted for consistent version management. The API compatibility badge checks for breaking changes relative to the current major version (`N.*.*`). 

***

Ply-rs is a small library built to read and write the PLY file format (also Polygon File Format, Stanford Triangle Format). The library supports all three subformats for both reading and writing: ASCII, binary big endian, and binary little endian. See [`examples/write_tetrahedron.rs`](examples/write_tetrahedron.rs) for a demonstration of writing binary PLY files.

It focuses on two main points:

- An easy and fast start.
- High performance if you're willing to do some things yourself.

## Getting started

This is the easiest way to read a ply file:

```rust
use ply_rs_bw as ply;

fn main() {
    // Set up a reader, in this case, a file.
    let path = "example_plys/greg_turk_example1_ok_ascii.ply";
    let mut f = std::fs::File::open(path).unwrap();

    // create a parser
    let p = ply::parser::Parser::<ply::ply::DefaultElement>::new();

    // use the parser: read the entire file
    let ply = p.read_ply(&mut f);

    // make sure it did work
    assert!(ply.is_ok());
    let ply = ply.unwrap();

    // proof that data has been read
    println!("Ply header: {:#?}", ply.header);
    println!("Ply data: {:?}", ply.payload);
}

```

If you need to inspect the header first and then continue parsing the payload, wrap your
`BufRead` source in `parser::Reader`. It keeps parser state such as file-relative line
tracking across `read_header`, `read_payload`, and `read_payload_for_element`.

```rust
use ply_rs_bw as ply;

fn main() {
    let path = "example_plys/greg_turk_example1_ok_ascii.ply";
    let file = std::fs::File::open(path).unwrap();
    let mut reader = ply::parser::Reader::new(std::io::BufReader::new(file));

    let parser = ply::parser::Parser::<ply::ply::DefaultElement>::new();
    let header = parser.read_header(&mut reader).unwrap();
    let payload = parser.read_payload(&mut reader, &header).unwrap();

    println!("Header: {:#?}", header);
    println!("Payload keys: {:?}", payload.keys().collect::<Vec<_>>());
}
```

Split parsing returns `ParseError`, which preserves file-relative line numbers and exposes
them via `ParseError::line()`.

### Write ply file

The simplest case of writing a ply file:

```rust
use ply_rs_bw::ply::{ Ply, DefaultElement };
use ply_rs_bw::writer::{ Writer };

/// Demonstrates simplest use case for reading from a file.
fn main() {
    // set up a target could also be a file
    let mut buf = Vec::<u8>::new();

    // create a ply object
    let mut ply = Ply::<DefaultElement>::new();

    // set up a writer
    let w = Writer::new();
    let written = w.write_ply(&mut buf, &mut ply).unwrap();
    println!("{} bytes written", written);
    println!("buffer size: {}", buf.len());

    // proof that data has been read

    // We can use `from_utf8` since PLY files only contain ASCII characters
    let output = String::from_utf8(buf).unwrap();
    println!("Written data:\n{}", output);
}
```

For more complicated examples, please see the [examples](examples/).

This implementation is mainly based on [these specifications](http://paulbourke.net/dataformats/ply/) with additions from [here](https://people.sc.fsu.edu/%7Ejburkardt/data/ply/ply.txt).
