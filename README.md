# Ply-rs
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Fuzz & Audit](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml)
[![Miri](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml)
[![crates.io](https://img.shields.io/crates/v/ply-rs_bw.svg)](https://crates.io/crates/ply-rs-bw)
[![API 3.x compatibility](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/ply-rs/rust.yml)](https://github.com/bourumir-wyngs/ply-rs/actions)
[![crates.io](https://img.shields.io/crates/l/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![crates.io](https://img.shields.io/crates/d/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![docs.rs](https://docs.rs/ply-rs-bw/badge.svg)](https://docs.rs/ply-rs-bw)

This is a forked version of the [ply-rs](https://github.com/Fluci/ply-rs) project that was created to address the use of `linked-hash-map` to resolve [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573). 

The crate has been renamed to `ply-rs-bw,` and minor issues were resolved to ensure compatibility with Rust 2024
edition. Additionally, an example has been added to demonstrate how to read PLY files with diverse field types
(e.g., `f32` vs `f64`, `u32` vs `i32`, etc.). Semantic versioning is now adopted for consistent version management. The API compatibility badge checks for breaking changes relative to the current major version (`N.*.*`). 

The reason we changed from 3.x to 4.x is that `ply_rs_bw::ply::PropertyType` and  `ply_rs_bw::ply::ScalarType` now implement `Copy` that semver checker considers the breaking change.

***

Ply-rs is a small library built to read and write the PLY file format (also Polygon File Format, Stanford Triangle Format). The library supports all three subformats for both reading and writing: ASCII, binary big endian, and binary little endian. See [`examples/write_tetrahedron.rs`](examples/write_tetrahedron.rs) for a demonstration of writing binary PLY files.

It focuses on two main points:

- An easy and fast start.
- High performance if you're willing to do some things yourself.

## Getting started

A PLY file consists of 3D points (vertices), which are defined separately, and faces that connect these vertices into triangles (or sometimes other polygons). Faces are typically short arrays of three indices that specify which vertices form a triangle.

From version 4.0.0, `ply-rs-bw` supports `serde`. This means you can use standard `#[derive(Serialize, Deserialize)]` on your structs to read and write PLY files, significantly reducing boilerplate. Data types are inferred for Rust types.

```rust
use serde::{Deserialize, Serialize};
use ply_rs_bw::{from_reader, to_writer};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Face {
    #[serde(rename = "vertex_indices")]
    indices: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Mesh {
    // Maps to "element vertex"
    #[serde(rename = "vertex")]
    vertices: Vec<Vertex>,
    // Maps to "element face"
    #[serde(rename = "face")]
    faces: Vec<Face>,
}

#[test]
fn test_serde_ply() {
    let vertices = vec![
        Vertex { x: 0.0, y: 0.0, z: 0.0 },
        Vertex { x: 1.0, y: 0.0, z: 0.0 },
        Vertex { x: 0.0, y: 1.0, z: 0.0 },
    ];
    let faces = vec![
        Face { indices: vec![0, 1, 2] },
    ];
    let mesh = Mesh { vertices, faces };

    // Write to a buffer (or file)
    let mut buf = Vec::new();
    to_writer(&mut buf, &mesh).unwrap();

    // Read back
    let read_mesh: Mesh = from_reader(&buf[..]).unwrap();
    
    assert_eq!(mesh, read_mesh);
}
```


For more complicated examples, please see the [examples](examples/).

This implementation is mainly based on [these specifications](http://paulbourke.net/dataformats/ply/) with additions from [here](https://people.sc.fsu.edu/%7Ejburkardt/data/ply/ply.txt).
