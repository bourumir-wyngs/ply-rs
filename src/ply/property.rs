//! Property model and access trait.
//!
//! PLY payload values are dynamically typed according to the header. This module
//! provides:
//! - [`Property`] as an enum covering all supported scalar and list payload values.
//! - [`ScalarType`] / [`PropertyType`] to describe the types declared in the header.
//! - [`PropertyAccess`] to allow parsing/writing payloads into custom data structures.

/// Scalar type used to encode properties in the payload.
///
/// For the translation to rust types, see individual documentation.
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ScalarType {
    /// Signed 8 bit integer, rust: `i8`.
    Char,
    /// Unsigned 8 bit integer, rust: `u8`.
    UChar,
    /// Signed 16 bit integer, rust: `i16`.
    Short,
    /// Unsigned 16 bit integer, rust: `u16`.
    UShort,
    /// Signed 32 bit integer, rust: `i32`.
    Int,
    /// Unsigned 32 bit integer, rust: `u32`.
    UInt,
    /// 32 bit floating point number, rust: `f32`.
    Float,
    /// 64 bit floating point number, rust: `f64`.
    Double,
}

/// Data type used to encode properties in the payload.
///
/// There are two possible types: scalars and lists.
/// Lists are a sequence of scalars with a leading integer value defining how many elements the list contains.
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PropertyType {
    /// Simple, "one-number" type.
    Scalar(ScalarType),
    /// Defines a sequence of scalars with the same type.
    ///
    /// First value is the index type which should be an integer variant.
    /// Encoded in ASCII, you always get the same number in the file (for example `32` or `17`).
    /// Hence, a good choice is mainly important for internal representation and binary encoding.
    /// The possible trade-off should be obvious:
    /// list length/flexibility against storage size. Though this obviously depends on your specific use case.
    ///
    /// Second value is the type of the list elements.
    List(ScalarType, ScalarType)
}

/// Wrapper used to implement a dynamic type system as required by the PLY file format.
#[derive(Debug, PartialEq, Clone)]
pub enum Property {
    /// Signed 8-bit integer scalar (`i8`).
    Char(i8),
    /// Unsigned 8-bit integer scalar (`u8`).
    UChar(u8),
    /// Signed 16-bit integer scalar (`i16`).
    Short(i16),
    /// Unsigned 16-bit integer scalar (`u16`).
    UShort(u16),
    /// Signed 32-bit integer scalar (`i32`).
    Int(i32),
    /// Unsigned 32-bit integer scalar (`u32`).
    UInt(u32),
    /// 32-bit floating point scalar (`f32`).
    Float(f32),
    /// 64-bit floating point scalar (`f64`).
    Double(f64),
    /// List of signed 8-bit integers.
    ListChar(Vec<i8>),
    /// List of unsigned 8-bit integers.
    ListUChar(Vec<u8>),
    /// List of signed 16-bit integers.
    ListShort(Vec<i16>),
    /// List of unsigned 16-bit integers.
    ListUShort(Vec<u16>),
    /// List of signed 32-bit integers.
    ListInt(Vec<i32>),
    /// List of unsigned 32-bit integers.
    ListUInt(Vec<u32>),
    /// List of 32-bit floating point values.
    ListFloat(Vec<f32>),
    /// List of 64-bit floating point values.
    ListDouble(Vec<f64>),
}

/// Provides setters and getters for the Parser and the Writer.
///
/// This trait allows you to create your own data structure for the case that the
/// default HashMap isn't efficient enough for you.
///
/// All setters and getters have default implementations that do nothing or at most return `None`.
///
/// Feel free only to implement what your application actually uses:
/// If you know, that you only expect unsigned shorts, don't bother about implementing signed shorts or floats, it won't be called.
///
/// The getters are named in congruence with `PropertyType` and `ScalarType`.
pub trait PropertyAccess {
    /// Creates a new, empty instance.
    fn new() -> Self;

    /// Sets the property value for the given property name.
    fn set_property(&mut self, _property_name: &str, _property: Property) {
        // By default, do nothing
        // Sombody might only want to write, no point in bothering him/her with setter implementations.
    }

    /// Returns the property value as a signed 8-bit integer (`char`).
    fn get_char(&self, _property_name: &str) -> Option<i8> {
        None
    }

    /// Returns the property value as an unsigned 8-bit integer (`uchar`).
    fn get_uchar(&self, _property_name: &str) -> Option<u8> {
        None
    }

    /// Returns the property value as a signed 16-bit integer (`short`).
    fn get_short(&self, _property_name: &str) -> Option<i16> {
        None
    }

    /// Returns the property value as an unsigned 16-bit integer (`ushort`).
    fn get_ushort(&self, _property_name: &str) -> Option<u16> {
        None
    }

    /// Returns the property value as a signed 32-bit integer (`int`).
    fn get_int(&self, _property_name: &str) -> Option<i32> {
        None
    }

    /// Returns the property value as an unsigned 32-bit integer (`uint`).
    fn get_uint(&self, _property_name: &str) -> Option<u32> {
        None
    }

    /// Returns the property value as a 32-bit floating point number (`float`).
    fn get_float(&self, _property_name: &str) -> Option<f32> {
        None
    }

    /// Returns the property value as a 64-bit floating point number (`double`).
    fn get_double(&self, _property_name: &str) -> Option<f64> {
        None
    }

    /// Returns the property value as a list of signed 8-bit integers.
    fn get_list_char(&self, _property_name: &str) -> Option<&[i8]> {
        None
    }

    /// Returns the property value as a list of unsigned 8-bit integers.
    fn get_list_uchar(&self, _property_name: &str) -> Option<&[u8]> {
        None
    }

    /// Returns the property value as a list of signed 16-bit integers.
    fn get_list_short(&self, _property_name: &str) -> Option<&[i16]> {
        None
    }

    /// Returns the property value as a list of unsigned 16-bit integers.
    fn get_list_ushort(&self, _property_name: &str) -> Option<&[u16]> {
        None
    }

    /// Returns the property value as a list of signed 32-bit integers.
    fn get_list_int(&self, _property_name: &str) -> Option<&[i32]> {
        None
    }

    /// Returns the property value as a list of unsigned 32-bit integers.
    fn get_list_uint(&self, _property_name: &str) -> Option<&[u32]> {
        None
    }

    /// Returns the property value as a list of 32-bit floating point numbers.
    fn get_list_float(&self, _property_name: &str) -> Option<&[f32]> {
        None
    }

    /// Returns the property value as a list of 64-bit floating point numbers.
    fn get_list_double(&self, _property_name: &str) -> Option<&[f64]> {
        None
    }
}

/// Defines whether a property is required or optional.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Requiredness {
    /// The property must be present in the header.
    Required,
    /// The property may be missing from the header.
    Optional,
}

/// Provides a schema for the properties expected by a data structure.
///
/// This is used by the parser to validate that all required properties are present
/// in the PLY header before attempting to read the payload.
pub trait PropertySchema {
    /// Returns a list of properties (name and requiredness) expected by this type.
    fn schema() -> Vec<(String, Requiredness)>;
}

/// Allows a type to be automatically parsed from a PLY element.
pub trait PlyAccess: PropertyAccess + PropertySchema {}
impl<T: PropertyAccess + PropertySchema> PlyAccess for T {}
