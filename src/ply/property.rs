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

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;

    impl PropertyAccess for Dummy {
        fn new() -> Self {
            Dummy
        }
    }

    #[test]
    fn test_enums() {
        // ScalarType
        let _ = ScalarType::Char;
        let _ = ScalarType::UChar;
        let _ = ScalarType::Short;
        let _ = ScalarType::UShort;
        let _ = ScalarType::Int;
        let _ = ScalarType::UInt;
        let _ = ScalarType::Float;
        let _ = ScalarType::Double;

        // PropertyType
        let _ = PropertyType::Scalar(ScalarType::Char);
        let _ = PropertyType::Scalar(ScalarType::UChar);
        let _ = PropertyType::Scalar(ScalarType::Short);
        let _ = PropertyType::Scalar(ScalarType::UShort);
        let _ = PropertyType::Scalar(ScalarType::Int);
        let _ = PropertyType::Scalar(ScalarType::UInt);
        let _ = PropertyType::Scalar(ScalarType::Float);
        let _ = PropertyType::Scalar(ScalarType::Double);
        let _ = PropertyType::List(ScalarType::UInt, ScalarType::Char);

        // Property
        let _ = Property::Char(i8::MIN);
        let _ = Property::UChar(u8::MAX);
        let _ = Property::Short(i16::MIN);
        let _ = Property::UShort(u16::MAX);
        let _ = Property::Int(i32::MIN);
        let _ = Property::UInt(u32::MAX);
        let _ = Property::Float(f32::NAN);
        let _ = Property::Double(f64::NAN);
        let _ = Property::ListChar(vec![i8::MIN]);
        let _ = Property::ListUChar(vec![u8::MAX]);
        let _ = Property::ListShort(vec![i16::MIN]);
        let _ = Property::ListUShort(vec![u16::MAX]);
        let _ = Property::ListInt(vec![i32::MIN]);
        let _ = Property::ListUInt(vec![u32::MAX]);
        let _ = Property::ListFloat(vec![f32::NAN]);
        let _ = Property::ListDouble(vec![f64::NAN]);
    }

    #[test]
    fn test_property_eq() {
        assert_eq!(Property::Char(0), Property::Char(0));
        assert_eq!(Property::UChar(0), Property::UChar(0));
        assert_eq!(Property::Short(0), Property::Short(0));
        assert_eq!(Property::UShort(0), Property::UShort(0));
        assert_eq!(Property::Int(0), Property::Int(0));
        assert_eq!(Property::UInt(0), Property::UInt(0));
        assert_eq!(Property::Float(0.0), Property::Float(0.0));
        assert_eq!(Property::Double(0.0), Property::Double(0.0));
        assert_eq!(Property::ListChar(vec![]), Property::ListChar(vec![]));
        assert_eq!(Property::ListUChar(vec![]), Property::ListUChar(vec![]));
        assert_eq!(Property::ListShort(vec![0]), Property::ListShort(vec![0]));
        assert_eq!(Property::ListUShort(vec![0]), Property::ListUShort(vec![0]));
        assert_eq!(Property::ListInt(vec![0]), Property::ListInt(vec![0]));
        assert_eq!(Property::ListUInt(vec![0]), Property::ListUInt(vec![0]));
        assert_eq!(Property::ListFloat(vec![0.0]), Property::ListFloat(vec![0.0]));
        assert_eq!(Property::ListDouble(vec![0.0]), Property::ListDouble(vec![0.0]));
    }

    #[test]
    fn test_property_nan_is_not_equal() {
        // Rust's float equality treats NaN != NaN; since `Property` derives `PartialEq`,
        // the same semantics apply here.
        assert_ne!(Property::Float(f32::NAN), Property::Float(f32::NAN));
        assert_ne!(Property::Double(f64::NAN), Property::Double(f64::NAN));
        assert_ne!(
            Property::ListFloat(vec![f32::NAN]),
            Property::ListFloat(vec![f32::NAN])
        );
        assert_ne!(
            Property::ListDouble(vec![f64::NAN]),
            Property::ListDouble(vec![f64::NAN])
        );
    }

    #[test]
    fn test_property_access_defaults() {
        let mut dummy = Dummy::new();
        dummy.set_property("foo", Property::Char(42));

        assert_eq!(dummy.get_char("foo"), None);
        assert_eq!(dummy.get_uchar("foo"), None);
        assert_eq!(dummy.get_short("foo"), None);
        assert_eq!(dummy.get_ushort("foo"), None);
        assert_eq!(dummy.get_int("foo"), None);
        assert_eq!(dummy.get_uint("foo"), None);
        assert_eq!(dummy.get_float("foo"), None);
        assert_eq!(dummy.get_double("foo"), None);
        assert_eq!(dummy.get_list_char("foo"), None);
        assert_eq!(dummy.get_list_uchar("foo"), None);
        assert_eq!(dummy.get_list_short("foo"), None);
        assert_eq!(dummy.get_list_ushort("foo"), None);
        assert_eq!(dummy.get_list_int("foo"), None);
        assert_eq!(dummy.get_list_uint("foo"), None);
        assert_eq!(dummy.get_list_float("foo"), None);
        assert_eq!(dummy.get_list_double("foo"), None);
    }
}