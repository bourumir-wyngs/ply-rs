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

This is a forked version of the [ply-rs](https://github.com/Fluci/ply-rs) project that was initially created to address the use of `linked-hash-map` to resolve [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573). After first making minor tweaks and adding examples, we currently made more major extensions by proposing to use macros for working with ply data structures.
***

Ply-rs is a small library built to read and write the PLY file format (also Polygon File Format, Stanford Triangle Format). The library supports all three subformats for both reading and writing: ASCII, binary big endian, and binary little endian. 

It focuses on two main points:

- An easy and fast start (now emphasized).
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
    #[ply(name = "vertex, vertices")] // multiple possible to support alternatives
    vertices: Vec<Vertex>,
    #[ply(name = "face")]
    faces: Vec<Face>,
}

#[test]
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
## Data types
Standard PLY data types are: `char (i8), uchar (u8), short (i16), ushort (u16), int (i32), uint (u32), float (f32), double (f64)`. Types like `u64, i64, u128`, and `i128` that this library supports are not part of the official standard and are considered extensions in some implementations.  

Example [colors_normals_cameras.rs](examples/colors_normals_cameras.rs) shows how to define read/write data beyond vertices and faces.

The old 3.x.x API is not removed and remains available if preferred. See [write_tetrahedron.rs](examples/write_tetrahedron.rs) and [read_diverse_field_types.rs](examples/read_diverse_field_types.rs) for how to use it. 

This implementation is mainly based on [these specifications](http://paulbourke.net/dataformats/ply/) with additions from [here](https://people.sc.fsu.edu/%7Ejburkardt/data/ply/ply.txt).
