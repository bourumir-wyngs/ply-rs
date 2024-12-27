# Ply-rs
[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/ply-rs)
[![crates.io](https://img.shields.io/crates/v/ply-rs_bw.svg)](https://crates.io/crates/ply-rs-bw)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/ply-rs/rust.yml)](https://github.com/bourumir-wyngs/ply-rs/actions)
[![crates.io](https://img.shields.io/crates/l/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![crates.io](https://img.shields.io/crates/d/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![docs.rs](https://docs.rs/ply-rs-bw/badge.svg)](https://docs.rs/ply-rs-bw)
[![Dependency Vulnerabilities](https://img.shields.io/endpoint?url=https%3A%2F%2Fapi-hooks.soos.io%2Fapi%2Fshieldsio-badges%3FbadgeType%3DDependencyVulnerabilities%26pid%3Dnzuz6lndt%26branchName%3Dmaster%26packageVersion%3Dlatest-stable)](https://app.soos.io/research/packages/rust/-/ply-rs-bw)

This is a forked version of the [ply-rs](https://github.com/Fluci/ply-rs) project that was created to address the use of `linked-hash-map` to resolve [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573). 

The crate has been renamed to `ply-rs-bw,` and minor issues were resolved to ensure compatibility with Rust 2021
edition. Additionally, an example has been added to demonstrate how to read PLY files with diverse field types
(e.g., `f32` vs `f64`, `u32` vs `i32`, etc.). Semantic versioning is now adopted for consistent version management.

Version 2.0.0 eliminates unnecessary cloning, as suggested by Nguyen Thuan Hung (see [pull request 'Optimise by not cloning key'](https://github.com/Fluci/ply-rs/pull/21/files)). While the scope of this change is limited, it modifies the signature of the public API trait `PropertyAccess`. The method:

`fn set_property(&mut self, _property_name: String, _property: Property)`
into
`fn set_property(&mut self, _property_name: &String, _property: Property)`

This breaking change necessitates incrementing the major version number. According to the author, this optimization reduces the time required to read 80,000 points from 450ms to 90ms, which is a significant improvement. The pull request has been reviewed and approved by our team.

***

Ply-rs is a small library built to read and write the PLY file format (also Polygon File Format, Standford Triangle Format). The library supports all three subformats: ASCII, big endian, and little endian.

It focuses on two main points:

- An easy and fast start.
- High performance if you're willing to do some things yourself.

## Getting started

### Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
ply-rs-bw = "1.0.1"
```

Add to your root:

```rust
extern crate ply_rs_bw;

fn main() {}
```

### Read a ply file

This is the easiest way to read a ply file:

```rust,no_run
extern crate ply_rs_bw;
use ply_rs_bw as ply;

/// Demonstrates simplest use case for reading from a file.
fn main() {
    //Set up a reader, in this case, a file.
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

### Write ply file

The simplest case of writing a ply file:

```rust
extern crate ply_rs_bw;
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
