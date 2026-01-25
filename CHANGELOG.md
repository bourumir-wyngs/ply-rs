# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2026-01-25

### Breaking changes
- `ply::PropertyAccess`: changed all property-name parameters from `&String` to `&str` (including `set_property` and all getters). This is a breaking change for crates that implement `PropertyAccess` (callers passing `&String` continue to work via deref coercion).
- `parser::ply_grammar::Line` (returned by `parser::Parser::read_header_line`): variant payloads changed:
  - `Line::Format((Encoding, Version))` -> `Line::Format((Encoding, Option<Version>))`
  - `Line::Element(ElementDef)` -> `Line::Element(Option<ElementDef>)`
  Downstream code pattern-matching on these variants must be updated to handle `Option`.

## [2.0.4] - 2026-01-25

### Added
- `Default` implementations for `Parser`, `Ply`, `Header`, and `Writer`
- `Parser::read_ply_header` convenience API for reading only the header (see docs about buffering)
- `SECURITY.md` with private vulnerability reporting guidance
- RustSec dependency audit GitHub Actions step (`rustsec/audit-check@v2`)
- Minimal fuzzing harness (`fuzz/`) with `read_ply` and `read_header` targets
- Miri checks (tests fixed to access data in miri-compatible way)
- Regression tests for crash-resistance and error-handling edge cases:
  - EOF before `end_header`
  - ASCII payload EOF
  - negative list length in binary payload
- Binary format write tests (working functionality existed but was not test-covered) 

### Changed
- Simplified code in `ply_grammar.rs` rule_element
- Code cleanup across library, examples, and tests (clippy warnings fixed)
- Improved error messages for some EOF cases to include element/count context

### Fixed
- Guarding against parser crashes caused by very large values for element size and version numbers that would not fit into `u64`
- Reduced panic possibility by replacing `.unwrap()` with `?` in ply_write module
- Fixed binary list-length parsing footgun: reject negative list lengths for signed index types and avoid unchecked `as usize` casts (prevents huge allocations/loops from crafted inputs)
- Improved EOF handling & diagnostics:
  - Header parsing now returns `UnexpectedEof` when the file ends before `ply` or `end_header`
  - ASCII payload parsing now errors on early EOF with element/count context
  - Binary payload parsing now wraps `UnexpectedEof` with element/count context

## [2.0.3] - 2025-11-12

### Added
- `#![forbid(unsafe_code)]` directive - the crate now guarantees no unsafe code
- More comprehensive test coverage
- Example for reading PLY files with diverse field types

### Changed
- Switched from `linked-hash-map` to `indexmap` for better maintenance and performance
- Replaced `skeptic` with `doc-comment` for documentation testing
- Performance improvements in parsing

### Fixed
- Support for uppercase 'E' in scientific notation parsing (contributed by @mattatz)

## [2.0.2] - 2025-03-08

### Changed
- Updated to Rust edition 2024
- Documentation improvements

## [2.0.1] - 2025-01-02

### Changed
- Minor optimizations
- Code cleanup (removed unnecessary semicolons)

## [2.0.0] - 2024-12-19

### Changed
- **BREAKING**: Modified `PropertyAccess` trait signature for performance optimization:
  - `fn set_property(&mut self, _property_name: String, _property: Property)` 
  - changed to `fn set_property(&mut self, _property_name: &String, _property: Property)`
- This optimization reduces time to read 80,000 points from 450ms to 90ms (credit: Nguyen Thuan Hung)

### Added
- Better PLY examples representing realistic rectangles

## [1.0.0] - 2024-12-07

### Added
- Initial release as `ply-rs-bw` fork
- Forked from [ply-rs](https://github.com/Fluci/ply-rs) to address [CVE-2020-25573](https://nvd.nist.gov/vuln/detail/CVE-2020-25573) in `linked-hash-map`

### Changed
- Renamed crate to `ply-rs-bw`
- Updated dependencies to resolve security vulnerability
- Adopted semantic versioning

---

## Original ply-rs History (pre-fork)

### [0.1.2] - 2019-01-15
- Updated dependency versions

### [0.1.1] - 2018-06-21
- Minor fixes and documentation updates

### [0.1.0] - 2017
- Initial release by Felice Serena
