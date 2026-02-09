//! PEG grammar for parsing PLY headers and ASCII data lines.
//!
//! This module is an internal implementation detail, but some types (e.g. [`Line`])
//! may surface through low-level APIs such as `Parser::read_header_line`.

use crate::ply::{ PropertyDef, PropertyType, ScalarType, Encoding, Version, Comment, ObjInfo,ElementDef };
#[derive(Debug, PartialEq, Clone)]
/// A single parsed header line.
///
/// This is used internally by the header parser to represent the different
/// kinds of statements that can occur in a PLY header.
pub enum Line {
    /// The `ply` magic number line.
    MagicNumber,
    /// A `format <encoding> <version>` line.
    Format((Encoding, Option<Version>)),
    /// A `comment ...` line.
    Comment(Comment),
    /// An `obj_info ...` line.
    ObjInfo(ObjInfo),
    /// An `element <name> <count>` line.
    Element(Option<ElementDef>),
    /// A `property ...` line.
    Property(PropertyDef),
    /// The `end_header` terminator line.
    EndHeader
}

peg::parser!{pub grammar grammar() for str {

/// Grammar for PLY header

pub rule number() -> &'input str
	= n:$(['0'..='9']+) { n }

rule space() = [' '|'\t']+

rule uint() -> Option<u64>
    = n:$(['0'..='9']+) {
        n.parse::<u64>().ok()
    }

rule ident() -> &'input str
	= s:$(['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'0'..='9'|'_'|'-']*) { s }

rule text() -> &'input str
	= s:$((!['\n'|'\r'][_])+) { s }

rule line_break()
	= "\r\n" / ['\n'|'\r']

rule scalar() -> ScalarType
	= "char"    { ScalarType::Char }
	/ "int8"    { ScalarType::Char }
	/ "uchar"   { ScalarType::UChar }
	/ "uint8"   { ScalarType::UChar }
	/ "short"   { ScalarType::Short }
	/ "int16"   { ScalarType::Short }
	/ "uint16"  { ScalarType::UShort }
	/ "ushort"  { ScalarType::UShort }
	/ "int32"   { ScalarType::Int }
	/ "int"     { ScalarType::Int }
	/ "uint32"  { ScalarType::UInt }
	/ "uint"    { ScalarType::UInt }
	/ "float32" { ScalarType::Float }
	/ "float64" { ScalarType::Double }
	/ "float"   { ScalarType::Float }
	/ "double"  { ScalarType::Double }

rule data_type() -> PropertyType
	= s:scalar()   { PropertyType::Scalar(s) }
	/ "list" space() it:scalar() space() t:scalar() {
		PropertyType::List(it, t)
	}

pub rule magic_number()
	= "ply"

pub rule format() -> (Encoding, Option<Version>)
	= "format" space() "ascii" space() v:version() { (Encoding::Ascii, v) }
	/ "format" space() "binary_big_endian" space() v:version() { (Encoding::BinaryBigEndian, v) }
	/ "format" space() "binary_little_endian" space() v:version() { (Encoding::BinaryLittleEndian, v) }

rule version() -> Option<Version>
    = maj:uint() "." min:uint() {{
        let maj = maj?;
        let min = min?;
        Some(Version {
            major: u16::try_from(maj).ok()?,
            minor: u8::try_from(min).ok()?,
        })
    }}

pub rule comment() -> Comment
	= "comment" space() c:text() {
		c.to_string()
	}
	/ "comment" space()? {
		String::new()
	}

pub rule obj_info() -> ObjInfo
	= "obj_info" space() c:text() {
	    c.to_string()
	}
	/ "obj_info" space()? {
	    String::new()
	}

pub rule element() -> Option<ElementDef>
    = "element" space() id:ident() space() n:uint() {{
        let mut e = ElementDef::new(id.to_owned());
        e.count = usize::try_from(n?).ok()?;
        Some(e)
    }}

pub rule property() -> PropertyDef
	= "property" space() data_type:data_type() space() id:ident() {
		PropertyDef::new(id.to_owned(), data_type)
	}

pub rule end_header()
	= "end_header"

pub rule line() -> Line
	= l:trimmed_line() space()? line_break()? { l }

rule trimmed_line() -> Line
	= magic_number() { Line::MagicNumber }
	/ end_header() { Line::EndHeader }
	/ v:format() { Line::Format(v) }
	/ v:obj_info() { Line::ObjInfo(v) }
	/ v:comment() { Line::Comment(v) }
	/ v:element() { Line::Element(v) }
	/ v:property() { Line::Property(v) }

rule any_number() -> &'input str
	= s:$(['-'|'+']? ['0'..='9']+("."['0'..='9']+)?(['e'|'E']['-'|'+']?['0'..='9']+)?) { s }

rule trimmed_data_line() -> Vec<&'input str>
	= any_number() ** space()

pub rule data_line() -> Vec<&'input str>
	= space()? l:trimmed_data_line() space()? line_break()? { l }

}}