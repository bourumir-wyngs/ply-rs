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

A PLY file consists of 3D points (vertices), which are defined separately, and faces that connect these vertices into triangles (or sometimes other polygons). Faces are typically short arrays of three indices that specify which vertices form a triangle From 4.0.0 the crate is macro-centric. Macros significantly reduce the boilerplate: 

```rust
use ply_rs_bw::{PlyRead, PlyWrite, ToPly, FromPly};

#[derive(Debug, Default, PlyRead, PlyWrite, PartialEq)]
struct Vertex {
    // we use maximal abstraction (ply types and names are inferred).
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Default, PlyRead, PlyWrite, PartialEq)]
struct Face {
    // we use maximum details
    #[ply(name = "vertex_indices", type = "uint", count = "uchar")]
    indices: Vec<u32>,
}

#[derive(Debug, ToPly, FromPly, PartialEq)]
struct Mesh {
    #[ply(name = "vertex")]
    vertices: Vec<Vertex>,
    #[ply(name = "face")]
    faces: Vec<Face>,
}

#[test]
#[cfg(not(miri))]
fn test_write_read_tetrahedron_macros() {
    // Create mesh
    let vertices = vec![
        Vertex { x: 1.0, y: 1.0, z: 1.0 },
        Vertex { x: 1.0, y: -1.0, z: -1.0 },
        Vertex { x: -1.0, y: 1.0, z: -1.0 },
        Vertex { x: -1.0, y: -1.0, z: 1.0 },
    ];

    let faces = vec![
        Face { indices: vec![0, 1, 2] },
        Face { indices: vec![0, 3, 1] },
        Face { indices: vec![0, 2, 3] },
        Face { indices: vec![1, 3, 2] },
    ];

    let mesh = Mesh { vertices, faces };

    // Write
    let mut buf = Vec::<u8>::new();
    mesh.write_ply(&mut buf).unwrap();

    // Read back
    let mut cursor = std::io::Cursor::new(buf);
    let read_mesh = Mesh::read_ply(&mut cursor).unwrap();

    // Assert
    assert_eq!(mesh, read_mesh);
}
```


For more complicated examples, please see the [examples](examples/).

This implementation is mainly based on [these specifications](http://paulbourke.net/dataformats/ply/) with additions from [here](https://people.sc.fsu.edu/%7Ejburkardt/data/ply/ply.txt).
