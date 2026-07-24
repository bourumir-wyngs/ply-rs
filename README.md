# Ply-rs
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Fuzz & Audit](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/ci.yml)
[![Miri](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/miri.yml)
[![crates.io](https://img.shields.io/crates/v/ply-rs_bw.svg)](https://crates.io/crates/ply-rs-bw)
[![API 4.x compatibility](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml/badge.svg)](https://github.com/bourumir-wyngs/ply-rs/actions/workflows/api-compat.yml)

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/ply-rs/rust.yml)](https://github.com/bourumir-wyngs/ply-rs/actions)
[![codecov](https://codecov.io/gh/bourumir-wyngs/ply-rs/graph/badge.svg)](https://codecov.io/gh/bourumir-wyngs/ply-rs)

[![crates.io](https://img.shields.io/crates/l/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![crates.io](https://img.shields.io/crates/d/ply-rs-bw.svg)](https://crates.io/crates/ply-rs-bw)
[![docs.rs](https://docs.rs/ply-rs-bw/badge.svg)](https://docs.rs/ply-rs-bw)
[![dependency status](https://deps.rs/crate/ply-rs-bw/latest/status.svg)](https://deps.rs/crate/ply-rs-bw/latest)
[![Socket Badge](https://badge.socket.dev/cargo/package/ply-rs-bw)](https://socket.dev/cargo/package/ply-rs-bw)

This is a forked version of the [ply-rs](https://github.com/Fluci/ply-rs) project that was created to address the use of `linked-hash-map`
to resolve [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573). Small changes in the API were made later to address practical use cases.
The API compatibility badge checks for breaking changes relative to the current major version (`N.*.*`).

***

Ply-rs is a small library built to read and write the PLY file format (also Polygon File Format, Stanford Triangle Format). The library supports all three subformats for both reading and writing: ASCII, binary big endian, and binary little endian. See [`examples/write_tetrahedron.rs`](examples/write_tetrahedron.rs) for a demonstration of writing binary PLY files.

## Getting started

### Format description

Vertices are points, usually described by `x`, `y`, and `z` coordinates. Faces
form polygons by storing lists of indices that refer to those vertices.

### Read and write a struct

When the schema is known, parsing directly into your final data structure avoids
allocating a [`DefaultElement`](https://docs.rs/ply-rs-bw/latest/ply_rs_bw/ply/type.DefaultElement.html)
property map for every vertex. This can improve performance for PLY files
containing many vertices. Vertex coordinates are
commonly encoded as `float` (`f32`) or `double` (`f64`), although integer scalar
types are also valid. The example below supports both floating-point types.

```rust
use ply_rs_bw::parser::{Parser, Reader};
use ply_rs_bw::ply::{Ply, PropertyAccess, PropertyAccessResult};
use ply_rs_bw::writer::Writer;

#[derive(Debug, Default)]
struct Vertex {
    x: f64,
    y: f64,
    z: f64,
}

impl Vertex {
    fn set_value(&mut self, name: &str, value: f64) -> PropertyAccessResult {
        match name {
            "x" => self.x = value,
            "y" => self.y = value,
            "z" => self.z = value,
            _ => return PropertyAccessResult::Ignored,
        }
        PropertyAccessResult::Set
    }

    fn value(&self, name: &str) -> Option<f64> {
        match name {
            "x" => Some(self.x),
            "y" => Some(self.y),
            "z" => Some(self.z),
            _ => None,
        }
    }
}

impl PropertyAccess for Vertex {
    fn new() -> Self {
        Self::default()
    }

    fn set_float(&mut self, name: &str, value: f32) -> PropertyAccessResult {
        self.set_value(name, f64::from(value))
    }

    fn set_double(&mut self, name: &str, value: f64) -> PropertyAccessResult {
        self.set_value(name, value)
    }

    // The parsed header determines which getter the writer calls.
    fn get_float(&self, name: &str) -> Option<f32> {
        self.value(name).map(|value| value as f32)
    }

    fn get_double(&self, name: &str) -> Option<f64> {
        self.value(name)
    }
}

#[derive(Debug, Default)]
struct Face {
    indices: Vec<i32>,
}

impl PropertyAccess for Face {
    fn new() -> Self {
        Self::default()
    }

    fn set_list_int(&mut self, name: &str, value: Vec<i32>) -> PropertyAccessResult {
        if name != "vertex_indices" {
            return PropertyAccessResult::Ignored;
        }
        self.indices = value;
        PropertyAccessResult::Set
    }
}

fn main() {
    // Each property may independently use `float` or `double`.
    // PLY content is often binary, but can also be ASCII (this library supports both)
    let input: &[u8] = b"ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property double y\n\
property float z\n\
element face 1\n\
property list uchar int vertex_indices\n\
end_header\n\
0 0 0\n\
1 0 0\n\
0 1 0\n\
3 0 1 2\n";

    let mut reader = Reader::new(input);
    let vertex_parser = Parser::<Vertex>::new();
    let header = vertex_parser.read_header(&mut reader).unwrap();

    let vertices = vertex_parser
        .read_payload_for_element(&mut reader, &header.elements["vertex"], &header)
        .unwrap();
    let faces = Parser::<Face>::new()
        .read_payload_for_element(&mut reader, &header.elements["face"], &header)
        .unwrap();

    println!("Vertices: {vertices:#?}");
    for face in faces {
        println!("Indices: {:?}", face.indices);
    }

    // Write a vertex-only PLY using the same Vertex type and parsed schema.
    let mut ply = Ply::<Vertex>::new();
    ply.header = header;
    ply.header.elements.shift_remove("face");
    ply.payload.insert("vertex".to_string(), vertices);
    let mut output = Vec::new();
    Writer::new().write_ply(&mut output, &mut ply).unwrap();
    println!("{}", String::from_utf8(output).unwrap());
}
```
For more complicated examples, please see the [examples](examples/).

This implementation is mainly based on [these specifications](http://paulbourke.net/dataformats/ply/) with additions from [here](https://people.sc.fsu.edu/%7Ejburkardt/data/ply/ply.txt).
