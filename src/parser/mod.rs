//! Reads ascii or binary data into a `Ply`.

use std::fmt::{self, Debug, Display, Formatter};
use std::io;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::result;

mod ply_grammar;

use self::ply_grammar::Line;
use self::ply_grammar::grammar;
use crate::util::LocationTracker;

const MAX_PREALLOCATED_SIZE: usize = 65_536;

/// Result type used by parser APIs.
pub type Result<T> = result::Result<T, ParseError>;

/// Structured parser error with optional file-relative line information.
#[derive(Debug)]
pub struct ParseError {
    kind: ErrorKind,
    line: Option<usize>,
    message: String,
}

impl ParseError {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            line: None,
            message: message.into(),
        }
    }

    fn with_line(kind: ErrorKind, line: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            line: Some(line),
            message: message.into(),
        }
    }

    /// Returns the underlying error kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the file-relative 1-based line number, when available.
    pub fn line(&self) -> Option<usize> {
        self.line
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "Line {}: {}", line, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl error::Error for ParseError {}

impl From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        Self::new(error.kind(), error.to_string())
    }
}

impl From<ParseError> for io::Error {
    fn from(error: ParseError) -> Self {
        io::Error::new(error.kind(), error)
    }
}

/// Stateful buffered reader used by the incremental parser API.
#[derive(Debug)]
pub struct Reader<T: BufRead> {
    inner: T,
    location: LocationTracker,
}

impl<T: BufRead> Reader<T> {
    /// Wraps a buffered reader while preserving parser state across header and payload reads.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            location: LocationTracker::new(),
        }
    }

    /// Returns a shared reference to the wrapped reader.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Returns the current 1-based line number tracked by the parser.
    pub fn line(&self) -> usize {
        self.location.line_index
    }

    /// Unwraps the reader.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

fn parse_ascii_rethrow<T, E: Debug>(
    location: &LocationTracker,
    line_str: &str,
    e: E,
    message: &str,
) -> Result<T> {
    Err(ParseError::with_line(
        ErrorKind::InvalidInput,
        location.line_index,
        format!("{message}\n\tString: '{line_str}'\n\tError: {:?}", e),
    ))
}
fn parse_ascii_error<T>(location: &LocationTracker, line_str: &str, message: &str) -> Result<T> {
    Err(ParseError::with_line(
        ErrorKind::InvalidInput,
        location.line_index,
        format!("{message}\n\tString: '{line_str}'"),
    ))
}

use std::marker::PhantomData;

/// Reads data given by a `Read` trait into `Ply` components.
///
/// In most cases `read_ply()` should suffice.
/// If you need finer control over the read process,
/// there are methods down to the line/element level.
///
/// # Examples
///
/// The most common case is probably to read from a file:
///
/// ```rust
/// # use ply_rs_bw::*;
/// // set up a reader, in this case a file.
/// let path = "example_plys/greg_turk_example1_ok_ascii.ply";
/// let mut f = std::fs::File::open(path).unwrap();
///
/// // create a parser
/// let p = parser::Parser::<ply::DefaultElement>::new();
///
/// // use the parser: read the entire file
/// let ply = p.read_ply(&mut f);
///
/// // Did it work?
/// assert!(ply.is_ok());
/// ```
///
/// If you need finer control, you can start splitting the read operations down to the line/element level.
///
/// In the following case we first read the header, and then continue with the payload.
/// We need to build a Ply ourselves.
///
/// ```rust
/// # use ply_rs_bw::*;
/// // set up a reader as before.
/// // let mut f = ... ;
/// # let path = "example_plys/greg_turk_example1_ok_ascii.ply";
/// # let f = std::fs::File::open(path).unwrap();
/// // Wrap the source in a stateful parser reader to preserve diagnostics
/// // and stream position across header/payload parsing.
/// let mut buf_read = parser::Reader::new(std::io::BufReader::new(f));
///
/// // create a parser
/// let p = parser::Parser::<ply::DefaultElement>::new();
///
/// // use the parser: read the header
/// let header = p.read_header(&mut buf_read);
/// // Did it work?
/// let header = header.unwrap();
///
/// // read the payload
/// let payload = p.read_payload(&mut buf_read, &header);
/// // Did it work?
/// let payload = payload.unwrap();
///
/// // May be create your own Ply:
/// let ply = ply::Ply {
///     header: header,
///     payload: payload,
/// };
///
/// println!("Ply: {:#?}", ply);
/// ```
///
#[derive(Debug)]
pub struct Parser<E: PropertyAccess> {
    phantom: PhantomData<E>,
}

impl<E: PropertyAccess> Default for Parser<E> {
    fn default() -> Self {
        Self::new()
    }
}

//use std::marker::PhantomData;
//use std::io::{ Read, BufReader };
use crate::ply::Ply;
use crate::ply::{Encoding, Header, Payload};

impl<E: PropertyAccess> Parser<E> {
    /// Creates a new `Parser<E>`, where `E` is the type to store the element data in.
    ///
    /// To get started quickly try `DefaultElement` from the `ply` module.
    pub fn new() -> Self {
        Parser {
            phantom: PhantomData,
        }
    }

    fn ceil_div(&self, dividend: usize, divisor: usize) -> usize {
        debug_assert!(divisor != 0);
        dividend / divisor + usize::from(!dividend.is_multiple_of(divisor))
    }

    fn cap_preallocated_size(&self, requested_size: usize) -> usize {
        if requested_size <= MAX_PREALLOCATED_SIZE {
            return requested_size;
        }

        let minimum_growth_factor = self.ceil_div(requested_size, MAX_PREALLOCATED_SIZE);
        let growth_factor = minimum_growth_factor
            .checked_next_power_of_two()
            .unwrap_or(usize::MAX);

        self.ceil_div(requested_size, growth_factor)
            .min(MAX_PREALLOCATED_SIZE)
    }

    /// Expects the complete content of a PLY file.
    ///
    /// A PLY file starts with "ply\n". `read_ply` reads until all elements have been read as
    /// defined in the header of the PLY file.
    pub fn read_ply<T: Read>(&self, source: &mut T) -> Result<Ply<E>> {
        let mut reader = Reader::new(BufReader::new(source));
        let header = self.read_header(&mut reader)?;
        let payload = self.read_payload(&mut reader, &header)?;
        let mut ply = Ply::new();
        ply.header = header;
        ply.payload = payload;
        Ok(ply)
    }

    /// Reads only the header portion of a PLY file (up to and including `end_header`).
    ///
    /// If you need to continue reading the payload from the same stream, prefer wrapping
    /// your reader in a [`Reader`] and calling [`Parser::read_header`], because this
    /// method creates an internal `BufReader` which may buffer past `end_header`.
    pub fn read_ply_header<T: Read>(&self, source: &mut T) -> Result<Header> {
        let mut reader = Reader::new(BufReader::new(source));
        self.read_header(&mut reader)
    }
}

// use ply::{ Header, Encoding };
use crate::ply::{Addable, Comment, ElementDef, KeyMap, ObjInfo, PropertyAccess, Version};
/*
use util::LocationTracker;
use super::Parser;
use super::Line;
use super::grammar;
use super::{parse_ascii_error, parse_ascii_rethrow};
use std::io;
use std::io::{ BufRead, ErrorKind, Result };
use std::result;
// */

// ////////////////////////
/// #Header
// ////////////////////////
impl<E: PropertyAccess> Parser<E> {
    /// Reads header until and including `end_header`.
    ///
    /// A PLY file starts with "ply\n". The header and the payload are separated by a line `end_header\n`.
    /// This method reads all header elements up to `end_header`.
    pub fn read_header<T: BufRead>(&self, reader: &mut Reader<T>) -> Result<Header> {
        self.__read_header(&mut reader.inner, &mut reader.location)
    }

    /// Parses a single PLY header line.
    ///
    /// This is a low-level helper that exposes the header grammar; most callers
    /// should use [`Parser::read_header`] or [`Parser::read_ply`].
    pub fn read_header_line(&self, line: &str) -> Result<Line> {
        match self.__read_header_line(line) {
            Ok(l) => Ok(l),
            Err(e) => Err(ParseError::new(
                ErrorKind::InvalidInput,
                format!("Couldn't parse line.\n\tString: {}\n\tError: {:?}", line, e),
            )),
        }
    }

    // private
    fn __read_header_line(
        &self,
        line_str: &str,
    ) -> result::Result<Line, peg::error::ParseError<peg::str::LineCol>> {
        grammar::line(line_str)
    }
    fn __read_header<T: BufRead>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
    ) -> Result<Header> {
        location.next_line();
        let mut line_str = String::new();
        if reader.read_line(&mut line_str)? == 0 {
            return Err(ParseError::new(
                ErrorKind::UnexpectedEof,
                "Unexpected end of file while reading magic number.",
            ));
        }
        match self.__read_header_line(&line_str) {
            Ok(Line::MagicNumber) => (),
            Ok(l) => {
                return parse_ascii_error(
                    location,
                    &line_str,
                    &format!("Expected magic number 'ply', but saw '{:?}'.", l),
                );
            }
            Err(e) => {
                return parse_ascii_rethrow(location, &line_str, e, "Expected magic number 'ply'.");
            }
        }

        let mut header_form_ver: Option<(Encoding, Option<Version>)> = None;
        let mut header_obj_infos = Vec::<ObjInfo>::new();
        let mut header_elements = KeyMap::<ElementDef>::new();
        let mut header_comments = Vec::<Comment>::new();
        location.next_line();
        'readlines: loop {
            line_str.clear();
            if reader.read_line(&mut line_str)? == 0 {
                return Err(ParseError::with_line(
                    ErrorKind::UnexpectedEof,
                    location.line_index,
                    "Unexpected end of file while reading header (missing 'end_header').",
                ));
            }
            let line = self.__read_header_line(&line_str);

            match line {
                Err(e) => {
                    return parse_ascii_rethrow(location, &line_str, e, "Couldn't parse line.");
                }
                Ok(Line::MagicNumber) => {
                    return parse_ascii_error(location, &line_str, "Unexpected 'ply' found.");
                }
                Ok(Line::Format(ref t)) => {
                    if let Some(f) = header_form_ver.as_ref() {
                        if f != t {
                            return parse_ascii_error(
                                location,
                                &line_str,
                                &format!(
                                    "Found contradicting format definition:\n\
                                    \tEncoding: {:?}, Version: {:?}\n\
                                    previous definition:\n\
                                    \tEncoding: {:?}, Version: {:?}",
                                    t.0, t.1, f.0, f.1
                                ),
                            );
                        }
                        if f.1.is_none() {
                            return parse_ascii_error(location, &line_str, "Invalid version");
                        }
                    } else {
                        header_form_ver = Some(*t);
                    }
                }
                Ok(Line::ObjInfo(ref o)) => header_obj_infos.push(o.clone()),
                Ok(Line::Comment(ref c)) => header_comments.push(c.clone()),
                Ok(Line::Element(ref e)) => {
                    if let Some(e) = e {
                        header_elements.add(e.clone())
                    } else {
                        return parse_ascii_error(location, &line_str, "Invalid element");
                    }
                }
                Ok(Line::Property(p)) => {
                    if header_elements.is_empty() {
                        return parse_ascii_error(
                            location,
                            &line_str,
                            &format!("Property '{:?}' found without preceding element.", p),
                        );
                    } else {
                        let (_, mut e) = header_elements.pop().unwrap();
                        e.properties.add(p);
                        header_elements.add(e);
                    }
                }
                Ok(Line::EndHeader) => {
                    location.next_line();
                    break 'readlines;
                }
            };
            location.next_line();
        }

        let (encoding, version) = if let Some((encoding, version)) = header_form_ver {
            (encoding, version)
        } else {
            return Err(ParseError::new(
                ErrorKind::InvalidInput,
                "No format line found.",
            ));
        };

        let version = if let Some(version) = version {
            version
        } else {
            return Err(ParseError::new(
                ErrorKind::InvalidInput,
                "Invalid version number.",
            ));
        };

        Ok(Header {
            encoding,
            version,
            obj_infos: header_obj_infos,
            comments: header_comments,
            elements: header_elements,
        })
    }
}

// //////////////////////
// # Payload
// //////////////////////
impl<E: PropertyAccess> Parser<E> {
    /// Reads payload. Encoding is chosen according to the encoding field in `header`.
    pub fn read_payload<T: BufRead>(
        &self,
        reader: &mut Reader<T>,
        header: &Header,
    ) -> Result<Payload<E>> {
        self.__read_payload(&mut reader.inner, &mut reader.location, header)
    }
    /// Reads entire list of elements from payload. Encoding is chosen according to `header`.
    ///
    /// Make sure to read the elements in the order as they are defined in the header.
    pub fn read_payload_for_element<T: BufRead>(
        &self,
        reader: &mut Reader<T>,
        element_def: &ElementDef,
        header: &Header,
    ) -> Result<Vec<E>> {
        match header.encoding {
            Encoding::Ascii => self.__read_ascii_payload_for_element(
                &mut reader.inner,
                &mut reader.location,
                element_def,
            ),
            Encoding::BinaryBigEndian => self.__read_big_endian_payload_for_element(
                &mut reader.inner,
                &mut reader.location,
                element_def,
            ),
            Encoding::BinaryLittleEndian => self.__read_little_endian_payload_for_element(
                &mut reader.inner,
                &mut reader.location,
                element_def,
            ),
        }
    }
    /// internal dispatcher based on the encoding
    fn __read_payload<T: BufRead>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
        header: &Header,
    ) -> Result<Payload<E>> {
        let mut payload = Payload::with_capacity(header.elements.len());

        // Use an iterator over `header.elements` and avoid repeated matching
        let read_payload_for_element = match header.encoding {
            Encoding::Ascii => Self::__read_ascii_payload_for_element,
            Encoding::BinaryBigEndian => Self::__read_big_endian_payload_for_element,
            Encoding::BinaryLittleEndian => Self::__read_little_endian_payload_for_element,
        };

        // Iterate over elements and process each with the selected reader
        for (key, element_def) in &header.elements {
            let elems = read_payload_for_element(self, reader, location, element_def)?;
            payload.insert(key.clone(), elems);
        }

        Ok(payload)
    }
}

use std::slice::Iter;
use std::str::FromStr;

use crate::ply::{PropertyType, ScalarType};
use std::error;

/// # Ascii
impl<E: PropertyAccess> Parser<E> {
    fn __read_ascii_payload_for_element<T: BufRead>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
        element_def: &ElementDef,
    ) -> Result<Vec<E>> {
        let mut elems = Vec::<E>::with_capacity(self.cap_preallocated_size(element_def.count));
        // Pre-allocate a reasonably sized buffer to avoid frequent growth for typical lines
        let mut line_str = String::with_capacity(128);
        for i in 0..element_def.count {
            line_str.clear();
            if reader.read_line(&mut line_str)? == 0 {
                return Err(ParseError::with_line(
                    ErrorKind::UnexpectedEof,
                    location.line_index,
                    format!(
                        "Unexpected end of file while reading element '{}' (expected {}, got {}).",
                        element_def.name, element_def.count, i,
                    ),
                ));
            }

            let element = match self.read_ascii_element(&line_str, element_def) {
                Ok(e) => e,
                Err(e) => {
                    return parse_ascii_rethrow(
                        location,
                        &line_str,
                        e,
                        "Couldn't read element line.",
                    );
                }
            };
            elems.push(element);
            location.next_line();
        }
        Ok(elems)
    }
    /// Read a single element. Assume it is encoded in ascii.
    ///
    /// Make sure all elements are parsed in the order they are defined in the header.
    pub fn read_ascii_element(&self, line: &str, element_def: &ElementDef) -> Result<E> {
        let elems = match grammar::data_line(line) {
            Ok(e) => e,
            Err(ref e) => {
                return Err(ParseError::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Couldn't parse element line.\n\tString: '{}'\n\tError: {}",
                        line, e
                    ),
                ));
            }
        };

        let mut elem_it: Iter<&str> = elems.iter();
        let mut vals = E::new();
        for (k, p) in &element_def.properties {
            self.__read_ascii_property_into(&mut vals, k, &p.data_type, &mut elem_it)?;
        }
        Ok(vals)
    }

    fn __next_ascii_token<'a>(
        &self,
        elem_iter: &mut Iter<'a, &str>,
        data_type: &PropertyType,
    ) -> Result<&'a str> {
        let s: &str = match elem_iter.next() {
            None => {
                return Err(ParseError::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Expected element of type '{:?}', but found nothing.",
                        data_type
                    ),
                ));
            }
            Some(x) => x,
        };
        Ok(s)
    }

    fn __read_ascii_property_into(
        &self,
        vals: &mut E,
        property_name: &str,
        data_type: &PropertyType,
        elem_iter: &mut Iter<&str>,
    ) -> Result<()> {
        let s = self.__next_ascii_token(elem_iter, data_type)?;

        match *data_type {
            PropertyType::Scalar(ref scalar_type) => match *scalar_type {
                ScalarType::Char => vals.set_char(property_name, self.parse(s)?),
                ScalarType::UChar => vals.set_uchar(property_name, self.parse(s)?),
                ScalarType::Short => vals.set_short(property_name, self.parse(s)?),
                ScalarType::UShort => vals.set_ushort(property_name, self.parse(s)?),
                ScalarType::Int => vals.set_int(property_name, self.parse(s)?),
                ScalarType::UInt => vals.set_uint(property_name, self.parse(s)?),
                ScalarType::Float => vals.set_float(property_name, self.parse(s)?),
                ScalarType::Double => vals.set_double(property_name, self.parse(s)?),
            },
            PropertyType::List(_, ref scalar_type) => {
                let count: usize = self.parse(s)?;
                match *scalar_type {
                    ScalarType::Char => {
                        if let Some(list) = vals.begin_list_char(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_char(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::UChar => {
                        if let Some(list) = vals.begin_list_uchar(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_uchar(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::Short => {
                        if let Some(list) = vals.begin_list_short(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_short(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::UShort => {
                        if let Some(list) = vals.begin_list_ushort(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_ushort(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::Int => {
                        if let Some(list) = vals.begin_list_int(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_int(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::UInt => {
                        if let Some(list) = vals.begin_list_uint(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_uint(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::Float => {
                        if let Some(list) = vals.begin_list_float(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_float(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                    ScalarType::Double => {
                        if let Some(list) = vals.begin_list_double(property_name, count) {
                            self.__read_ascii_list_into(elem_iter, count, list)?;
                        } else {
                            vals.set_list_double(
                                property_name,
                                self.__read_ascii_list(elem_iter, count)?,
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse<D: FromStr>(&self, s: &str) -> Result<D>
    where
        <D as FromStr>::Err: error::Error + Send + Sync + 'static,
    {
        let v = s.parse();
        match v {
            Ok(r) => Ok(r),
            Err(e) => Err(ParseError::new(
                ErrorKind::InvalidInput,
                format!("Parse error.\n\tValue: '{}'\n\tError: {:?}, ", s, e),
            )),
        }
    }

    fn __prepare_list<D>(&self, out: &mut Vec<D>, count: usize) {
        out.clear();
        let desired_capacity = self.cap_preallocated_size(count);
        if out.capacity() < desired_capacity {
            out.reserve(desired_capacity - out.capacity());
        }
    }

    fn __read_ascii_list<D: FromStr>(
        &self,
        elem_iter: &mut Iter<&str>,
        count: usize,
    ) -> Result<Vec<D>>
    where
        <D as FromStr>::Err: error::Error + Send + Sync + 'static,
    {
        let mut out = Vec::new();
        self.__read_ascii_list_into(elem_iter, count, &mut out)?;
        Ok(out)
    }

    fn __read_ascii_list_into<D: FromStr>(
        &self,
        elem_iter: &mut Iter<&str>,
        count: usize,
        out: &mut Vec<D>,
    ) -> Result<()>
    where
        <D as FromStr>::Err: error::Error + Send + Sync + 'static,
    {
        self.__prepare_list(out, count);
        for i in 0..count {
            let s = match elem_iter.next() {
                Some(s) => s,
                None => {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        format!("Expected {} list elements, but found only {}.", count, i),
                    ));
                }
            };
            match s.parse() {
                Ok(v) => out.push(v),
                Err(err) => {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        format!("Couldn't parse element at index {}: {:?}", i, err),
                    ));
                }
            }
        }
        Ok(())
    }
}

// //////////////////////////////////////
// # Binary
// //////////////////////////////////////
/*
use std::io;
use std::io::{ Read, Result, ErrorKind };
use std::str::FromStr;
use std::error;
use std::marker;
use ply::{ PropertyAccess, ElementDef, PropertyType, Property, ScalarType };
use util::LocationTracker;
use super::Parser;
*/
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use peg;

/// # Binary
impl<E: PropertyAccess> Parser<E> {
    /// Reads a single element as declared in `element_def`. Assumes big endian encoding.
    ///
    /// Make sure all elements are parsed in the order they are defined in the header.
    pub fn read_big_endian_element<T: Read>(
        &self,
        reader: &mut T,
        element_def: &ElementDef,
    ) -> Result<E> {
        // Reduce coupling with ByteOrder
        self.__read_binary_element::<T, BigEndian>(reader, element_def)
    }
    /// Reads a single element as declared in `element_def`. Assumes little endian encoding.
    ///
    /// Make sure all elements are parsed in the order they are defined in the header.
    pub fn read_little_endian_element<T: Read>(
        &self,
        reader: &mut T,
        element_def: &ElementDef,
    ) -> Result<E> {
        // Reduce coupling with ByteOrder
        self.__read_binary_element::<T, LittleEndian>(reader, element_def)
    }

    /// internal wrapper
    fn __read_big_endian_payload_for_element<T: Read>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
        element_def: &ElementDef,
    ) -> Result<Vec<E>> {
        self.__read_binary_payload_for_element::<T, BigEndian>(reader, location, element_def)
    }
    fn __read_little_endian_payload_for_element<T: Read>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
        element_def: &ElementDef,
    ) -> Result<Vec<E>> {
        self.__read_binary_payload_for_element::<T, LittleEndian>(reader, location, element_def)
    }

    fn __read_binary_payload_for_element<T: Read, B: ByteOrder>(
        &self,
        reader: &mut T,
        location: &mut LocationTracker,
        element_def: &ElementDef,
    ) -> Result<Vec<E>> {
        let mut elems = Vec::<E>::with_capacity(self.cap_preallocated_size(element_def.count));
        for i in 0..element_def.count {
            let element = self
                .__read_binary_element::<T, B>(reader, element_def)
                .map_err(|e| {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        ParseError::with_line(
                            ErrorKind::UnexpectedEof,
                            location.line_index,
                            format!(
                                "Unexpected end of file while reading binary element '{}' (expected {}, got {}).\n\tError: {}",
                                element_def.name, element_def.count, i, e,
                            ),
                        )
                    } else {
                        e
                    }
                })?;
            elems.push(element);
            location.next_line();
        }
        Ok(elems)
    }
    fn __read_binary_element<T: Read, B: ByteOrder>(
        &self,
        reader: &mut T,
        element_def: &ElementDef,
    ) -> Result<E> {
        let mut raw_element = E::new();

        for (k, p) in &element_def.properties {
            self.__read_binary_property_into::<T, B>(reader, &mut raw_element, k, &p.data_type)?;
        }
        Ok(raw_element)
    }

    fn __read_binary_property_into<T: Read, B: ByteOrder>(
        &self,
        reader: &mut T,
        raw_element: &mut E,
        property_name: &str,
        data_type: &PropertyType,
    ) -> Result<()> {
        match *data_type {
            PropertyType::Scalar(ref scalar_type) => match *scalar_type {
                ScalarType::Char => raw_element.set_char(property_name, reader.read_i8()?),
                ScalarType::UChar => raw_element.set_uchar(property_name, reader.read_u8()?),
                ScalarType::Short => raw_element.set_short(property_name, reader.read_i16::<B>()?),
                ScalarType::UShort => {
                    raw_element.set_ushort(property_name, reader.read_u16::<B>()?)
                }
                ScalarType::Int => raw_element.set_int(property_name, reader.read_i32::<B>()?),
                ScalarType::UInt => raw_element.set_uint(property_name, reader.read_u32::<B>()?),
                ScalarType::Float => raw_element.set_float(property_name, reader.read_f32::<B>()?),
                ScalarType::Double => {
                    raw_element.set_double(property_name, reader.read_f64::<B>()?)
                }
            },
            PropertyType::List(ref index_type, ref property_type) => {
                let count = self.__read_binary_list_count::<T, B>(reader, index_type)?;
                match *property_type {
                    ScalarType::Char => {
                        if let Some(list) = raw_element.begin_list_char(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_i8()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_char(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_i8()?), count)?,
                            );
                        }
                    }
                    ScalarType::UChar => {
                        if let Some(list) = raw_element.begin_list_uchar(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_u8()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_uchar(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_u8()?), count)?,
                            );
                        }
                    }
                    ScalarType::Short => {
                        if let Some(list) = raw_element.begin_list_short(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_i16::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_short(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_i16::<B>()?), count)?,
                            );
                        }
                    }
                    ScalarType::UShort => {
                        if let Some(list) = raw_element.begin_list_ushort(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_u16::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_ushort(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_u16::<B>()?), count)?,
                            );
                        }
                    }
                    ScalarType::Int => {
                        if let Some(list) = raw_element.begin_list_int(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_i32::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_int(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_i32::<B>()?), count)?,
                            );
                        }
                    }
                    ScalarType::UInt => {
                        if let Some(list) = raw_element.begin_list_uint(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_u32::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_uint(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_u32::<B>()?), count)?,
                            );
                        }
                    }
                    ScalarType::Float => {
                        if let Some(list) = raw_element.begin_list_float(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_f32::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_float(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_f32::<B>()?), count)?,
                            );
                        }
                    }
                    ScalarType::Double => {
                        if let Some(list) = raw_element.begin_list_double(property_name, count) {
                            self.__read_binary_list_into(
                                reader,
                                |r| Ok(r.read_f64::<B>()?),
                                count,
                                list,
                            )?;
                        } else {
                            raw_element.set_list_double(
                                property_name,
                                self.__read_binary_list(reader, |r| Ok(r.read_f64::<B>()?), count)?,
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn __read_binary_list_count<T: Read, B: ByteOrder>(
        &self,
        reader: &mut T,
        index_type: &ScalarType,
    ) -> Result<usize> {
        match *index_type {
            ScalarType::Char => {
                let v = reader.read_i8()?;
                if v < 0 {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length cannot be negative (i8).",
                    ));
                }
                usize::try_from(v as i64).map_err(|_| {
                    ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length does not fit into usize.",
                    )
                })
            }
            ScalarType::UChar => Ok(usize::from(reader.read_u8()?)),
            ScalarType::Short => {
                let v = reader.read_i16::<B>()?;
                if v < 0 {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length cannot be negative (i16).",
                    ));
                }
                usize::try_from(v as i64).map_err(|_| {
                    ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length does not fit into usize.",
                    )
                })
            }
            ScalarType::UShort => Ok(usize::from(reader.read_u16::<B>()?)),
            ScalarType::Int => {
                let v = reader.read_i32::<B>()?;
                if v < 0 {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length cannot be negative (i32).",
                    ));
                }
                usize::try_from(v as i64).map_err(|_| {
                    ParseError::new(
                        ErrorKind::InvalidInput,
                        "List length does not fit into usize.",
                    )
                })
            }
            ScalarType::UInt => usize::try_from(reader.read_u32::<B>()?).map_err(|_| {
                ParseError::new(
                    ErrorKind::InvalidInput,
                    "List length does not fit into usize.",
                )
            }),
            ScalarType::Float => Err(ParseError::new(
                ErrorKind::InvalidInput,
                "Index of list must be an integer type, float declared in ScalarType.",
            )),
            ScalarType::Double => Err(ParseError::new(
                ErrorKind::InvalidInput,
                "Index of list must be an integer type, double declared in ScalarType.",
            )),
        }
    }

    fn __read_binary_list<T: Read, D, F>(
        &self,
        reader: &mut T,
        read_from: F,
        count: usize,
    ) -> Result<Vec<D>>
    where
        F: Fn(&mut T) -> Result<D>,
    {
        let mut list = Vec::new();
        self.__read_binary_list_into(reader, read_from, count, &mut list)?;
        Ok(list)
    }

    fn __read_binary_list_into<T: Read, D, F>(
        &self,
        reader: &mut T,
        read_from: F,
        count: usize,
        list: &mut Vec<D>,
    ) -> Result<()>
    where
        F: Fn(&mut T) -> Result<D>,
    {
        self.__prepare_list(list, count);
        for i in 0..count {
            let value: D = match read_from(reader) {
                Err(e) => {
                    return Err(ParseError::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Couldn't find a list element at index {}.\n\tError: {:?}",
                            i, e
                        ),
                    ));
                }
                Ok(x) => x,
            };
            list.push(value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Line;
    use super::grammar as g;
    use crate::parser::{Parser, Reader};
    use crate::ply::{
        Addable, DefaultElement, ElementDef, Encoding, KeyMap, PropertyDef, PropertyType,
        ScalarType, Version,
    };
    macro_rules! assert_ok {
        ($e:expr) => {
            match $e {
                Ok(obj) => (obj),
                Err(e) => panic!("{}", e),
            }
        };
        ($e:expr , $o:expr) => {
            let obj = assert_ok!($e);
            assert_eq!(obj, $o);
        };
    }
    macro_rules! assert_err {
        ($e:expr) => {
            let result = $e;
            assert!(result.is_err());
        };
    }
    #[test]
    fn parser_header_ok() {
        let p = Parser::<DefaultElement>::new();
        let txt = "ply\nformat ascii 1.0\nend_header\n";
        let mut bytes = Reader::new(txt.as_bytes());
        assert_ok!(p.read_header(&mut bytes));

        let txt = "ply\n\
        format ascii 1.0\n\
        element vertex 8\n\
        property float x\n\
        property float y\n\
        element face 6\n\
        property list uchar int vertex_index\n\
        end_header\n";
        let mut bytes = Reader::new(txt.as_bytes());
        assert_ok!(p.read_header(&mut bytes));
    }
    #[test]
    fn parser_demo_ok() {
        let txt = "ply\nformat ascii 1.0\nend_header\n";
        let mut bytes = Reader::new(txt.as_bytes());
        let p = Parser::<DefaultElement>::new();
        assert_ok!(p.read_header(&mut bytes));

        let txt = "ply\n\
        format ascii 1.0\n\
        element vertex 1\n\
        property float x\n\
        end_header\n
        6.28318530718"; // no newline at end!
        let mut bytes = Reader::new(txt.as_bytes());
        assert_ok!(p.read_header(&mut bytes));
    }
    #[test]
    fn parser_single_elements_ok() {
        let txt = "ply\r\n\
        format ascii 1.0\r\n\
        comment Hi, I'm your friendly comment.\r\n\
        obj_info And I'm your object information.\r\n\
        element point 2\r\n\
        property int x\r\n\
        property int y\r\n\
        end_header\r\n\
        -7 5\r\n\
        2 4\r\n";
        let mut bytes = txt.as_bytes();
        let p = Parser::<DefaultElement>::new();
        assert_ok!(p.read_ply(&mut bytes));
    }
    #[test]
    fn cap_preallocated_size_caps_large_requests() {
        let p = Parser::<DefaultElement>::new();

        assert_eq!(p.cap_preallocated_size(0), 0);
        assert_eq!(p.cap_preallocated_size(4), 4);
        assert_eq!(p.cap_preallocated_size(65_536), 65_536);
        assert_eq!(p.cap_preallocated_size(65_537), 32_769);
        assert_eq!(p.cap_preallocated_size(100_000), 50_000);
        assert_eq!(p.cap_preallocated_size(1_048_577), 32_769);
        assert_eq!(p.cap_preallocated_size(usize::MAX), 65_536);
    }
    #[test]
    fn read_property_ok() {
        let p = Parser::<DefaultElement>::new();
        let txt = "0 1 2 3";
        let mut prop = KeyMap::<PropertyDef>::new();
        prop.add(PropertyDef::new(
            "a".to_string(),
            PropertyType::Scalar(ScalarType::Char),
        ));
        prop.add(PropertyDef::new(
            "b".to_string(),
            PropertyType::Scalar(ScalarType::UChar),
        ));
        prop.add(PropertyDef::new(
            "c".to_string(),
            PropertyType::Scalar(ScalarType::Short),
        ));
        prop.add(PropertyDef::new(
            "d".to_string(),
            PropertyType::Scalar(ScalarType::UShort),
        ));
        let mut elem_def = ElementDef::new("dummy".to_string());
        elem_def.properties = prop;

        let properties = p.read_ascii_element(txt, &elem_def);
        assert!(properties.is_ok(), "{}", format!("error: {:?}", properties));
    }
    #[test]
    fn magic_number_ok() {
        assert_ok!(g::magic_number("ply"));
    }
    #[test]
    fn magic_number_err() {
        assert_err!(g::magic_number("py"));
        assert_err!(g::magic_number("plyhi"));
        assert_err!(g::magic_number("hiply"));
        assert_err!(g::magic_number(" ply"));
        assert_err!(g::magic_number("ply "));
    }
    #[test]
    fn format_ok() {
        assert_ok!(
            g::format("format ascii 1.0"),
            (Encoding::Ascii, Some(Version { major: 1, minor: 0 }))
        );
        assert_ok!(
            g::format("format binary_big_endian 2.1"),
            (
                Encoding::BinaryBigEndian,
                Some(Version { major: 2, minor: 1 })
            )
        );
        assert_ok!(
            g::format("format binary_little_endian 1.0"),
            (
                Encoding::BinaryLittleEndian,
                Some(Version { major: 1, minor: 0 })
            )
        );
        assert_ok!(
            g::format("format binary_little_endian 1.99999999999999999999999999999999999999999999"),
            (Encoding::BinaryLittleEndian, None)
        );
    }
    #[test]
    fn format_err() {
        assert_err!(g::format("format asciii 1.0"));
        assert_err!(g::format("format ascii -1.0"));
        assert_err!(g::format("format ascii 1.0.3"));
        assert_err!(g::format("format ascii 1."));
        assert_err!(g::format("format ascii 1"));
        assert_err!(g::format("format ascii 1.0a"));
    }
    #[test]
    fn comment_ok() {
        assert_ok!(g::comment("comment hi"), "hi");
        assert_ok!(
            g::comment("comment   hi, I'm a comment!"),
            "hi, I'm a comment!"
        );
        assert_ok!(g::comment("comment "), "");
        assert_ok!(g::comment("comment\t"), "");
        assert_ok!(g::comment("comment"), "");
        assert_ok!(g::comment("comment\t"), "");
        assert_ok!(g::comment("comment\thi"), "hi");
    }
    #[test]
    fn comment_err() {
        assert_err!(g::comment("commentt"));
        assert_err!(g::comment("comment\n"));
        assert_err!(g::comment("comment hi\na comment"));
        assert_err!(g::comment("comment hi\r\na comment"));
        assert_err!(g::comment("comment hi\ra comment"));
    }
    #[test]
    fn obj_info_ok() {
        assert_ok!(g::obj_info("obj_info Hi, I can help."), "Hi, I can help.");
        assert_ok!(g::obj_info("obj_info"), "");
        assert_ok!(g::obj_info("obj_info "), "");
        assert_ok!(g::obj_info("obj_info\t"), "");
    }
    #[test]
    fn obj_info_err() {
        assert_err!(g::obj_info("obj_info\n"));
    }
    #[test]
    fn element_ok() {
        let e = Some(ElementDef {
            name: "vertex".to_string(),
            count: 8,
            properties: Default::default(),
        });
        assert_ok!(g::element("element vertex 8"), e);
    }
    #[test]
    fn element_err() {
        assert_err!(g::comment("element 8 vertex"));
    }
    #[test]
    fn property_ok() {
        assert_ok!(
            g::property("property char c"),
            PropertyDef::new("c".to_string(), PropertyType::Scalar(ScalarType::Char))
        );
    }
    #[test]
    fn property_list_ok() {
        assert_ok!(
            g::property("property list uchar int c"),
            PropertyDef::new(
                "c".to_string(),
                PropertyType::List(ScalarType::UChar, ScalarType::Int)
            )
        );
    }
    #[test]
    fn line_ok() {
        assert_ok!(g::line("ply "), Line::MagicNumber);
        assert_ok!(
            g::line("format ascii 1.0 "),
            Line::Format((Encoding::Ascii, Some(Version { major: 1, minor: 0 })))
        );
        assert_ok!(g::line("comment a very nice comment "));
        assert_ok!(g::line("element vertex 8 "));
        assert_ok!(g::line("property float x "));
        assert_ok!(g::line("element face 6 "));
        assert_ok!(g::line("property list uchar int vertex_index "));
        assert_ok!(g::line("end_header "));
    }
    #[test]
    fn line_breaks_ok() {
        assert_ok!(g::line("ply \n"), Line::MagicNumber); // Unix, Mac OS X
        assert_ok!(g::line("ply \r"), Line::MagicNumber); // Mac pre OS X
        assert_ok!(g::line("ply \r\n"), Line::MagicNumber); // Windows
    }
    #[test]
    fn data_line_ok() {
        assert_ok!(
            g::data_line("+7 -7 7 +5.21 -5.21 5.21 +0 -0 0 \r\n"),
            vec!["+7", "-7", "7", "+5.21", "-5.21", "5.21", "+0", "-0", "0"]
        );
        assert_ok!(g::data_line("034 8e3 8e-3"), vec!["034", "8e3", "8e-3"]);
        assert_ok!(g::data_line(""), Vec::<&str>::new());
    }
    #[test]
    fn data_line_err() {
        assert_err!(g::data_line("++3"));
        assert_err!(g::data_line("+-3"));
        assert_err!(g::data_line("five"));
    }
}
