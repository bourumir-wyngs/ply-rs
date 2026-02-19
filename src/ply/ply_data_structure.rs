//! Core data structures representing a PLY file.
//!
//! This module defines [`Ply`], its [`Header`], and the type definitions needed to
//! describe element/property declarations.

use std::fmt::{ Display, Formatter };
use std::fmt;
use super::PropertyType;
use super::KeyMap;
use super::PropertyAccess;

/// Models all necessary information to interact with a PLY file.
///
/// The generic parameter `E` is the element type used to store the payload data.
#[derive(Debug, Clone, PartialEq)]
pub struct Ply<E: PropertyAccess> {
    /// All header information found in a PLY file.
    pub header: Header,
    /// The payload found after the `end_header` line in a PLY file.
    ///
    /// One line in an ASCII PLY file corresponds to a single element.
    /// The payload groups elements with the same type together in a vector.
    ///
    /// # Examples
    ///
    /// Assume you have a `Ply` object called `ply` and want to access the third `point` element:
    ///
    /// ```rust,no_run
    /// # use ply_rs_bw::ply::{Ply, DefaultElement};
    /// # let ply = Ply::<DefaultElement>::new();
    /// // get ply from somewhere ...
    /// let ref a_point = ply.payload["point"][2];
    /// let ref a_point_x = ply.payload["point"][2]["x"];
    /// ```
    pub payload: Payload<E>,
}

impl<E: PropertyAccess> Default for Ply<E> {
    fn default() -> Self {
        Self::new()
    }
}
impl<E: PropertyAccess> Ply<E> {
    /// Creates a new `Ply<E>`.
    pub fn new() -> Self {
        Ply::<E> {
            header: Header::new(),
            payload: Payload::new(),
        }
    }
}

// Header Types

/// Models the header of a PLY file.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Header {
    /// In which format is the payload encoded?
    ///
    /// Ascii produces human readable files,
    /// while binary encoding lets you choose between big and little endian.
    pub encoding: Encoding,
    /// Which file format standard is used?
    ///
    /// The only existing standard is 1.0.
    pub version: Version,
    /// Arbitrary object metadata lines (`obj_info ...`) as found in the header.
    pub obj_infos: Vec<ObjInfo>,
    /// Ordered map of elements as they appear in the payload.
    pub elements: KeyMap<ElementDef>,
    /// File comments.
    pub comments: Vec<Comment>,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    /// Constructs an empty `Header` using ASCII encoding and version 1.0.
    /// No object information, elements, or comments are set.
    pub fn new() -> Self {
        Header {
            encoding: Encoding::Ascii,
            version: Version { major: 1, minor: 0 },
            obj_infos: Vec::new(),
            elements: KeyMap::new(),
            comments: Vec::new(),
        }
    }
}

/// Alias to give object information an explicit type.
pub type ObjInfo = String;

/// Alias to give comments an explicit type.
pub type Comment = String;

/// Models a version number.
///
/// At time of writing, the only existing version for a PLY file is "1.0".
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Version {
    /// Major version number.
    pub major: u16,
    /// Minor version number.
    pub minor: u8,
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(&format!("{}.{}", self.major, self.minor))
    }
}

/// Models possible encoding standards for the payload.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Encoding {
    /// Write numbers in their ascii representation (e.g. -13, 6.28, etc.).
    /// Properties are separated by spaces and elements are separated by line breaks.
    Ascii,
    /// Encode payload using big endian.
    BinaryBigEndian,
    /// Encode payload using little endian.
    BinaryLittleEndian,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(
            match *self {
                Encoding::Ascii => "ascii",
                Encoding::BinaryBigEndian => "binary_big_endian",
                Encoding::BinaryLittleEndian => "binary_little_endian",
            }
        )
    }
}

/// Models the definition of an element.
///
/// Elements describe single entities consisting of different properties.
/// A single point is an element.
/// We might model it as consisting of three coordinates: x, y, and z.
/// Usually, one finds a list of elements in a ply file.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ElementDef {
    /// Name of the element.
    ///
    /// Each element within a PLY file needs a unique name.
    /// There are common conventions like using "vertex" and "face" to assure interoperability between applications.
    /// For further information, please consult your target applications or the [original specification](http://paulbourke.net/dataformats/ply/).
    pub name: String,
    /// Describes, how many elements appear in a PLY file.
    ///
    /// The `count` is used when reading since we need to know how many elements we should interprete as having this type.
    /// The `count` is also needed for writing, since it will be written to the header.
    pub count: usize,
    /// An element is modeled by multiple properties, those are named values or lists.
    ///
    /// # Examples
    ///
    /// - Point: We can define a point by its three coordinates. Hence we have three properties: x, y, and z. Reasonable types would be float or double.
    /// - Polygon: A polygon can be defined as a list of points. Since the points are stored in a list, we can define a list of indices. Good types would be some of the unsigned integer lists.
    pub properties: KeyMap<PropertyDef>,
}
impl ElementDef {
    /// Creates a new element definition.
    ///
    /// The name should be unique for each element in a PLY file.
    ///
    /// You should never need to set `count` manually, since it is set by the consistency check (see `make_consistent()` of `Ply`).
    ///
    /// No properties are set.
    pub fn new(name: String) -> Self {
        ElementDef {
            name,
            count: 0,
            properties: KeyMap::new(),
        }
    }
}

/// Defines a property of an element.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PropertyDef {
    /// Unique name of property.
    ///
    /// The name should be unique for each property of the same element.
    pub name: String,
    /// Data type of the property:
    /// You can have simple scalars (ints, floats, etc.) or lists of scalars.
    /// In the case of lists you need to decide in which type you want to store the list length and what type to use for the list elements.
    pub data_type: PropertyType,
}

impl PropertyDef {
    /// Creates a new property definition.
    pub fn new(name: String, data_type: PropertyType) -> Self {
        PropertyDef {
            name,
            data_type,
        }
    }
}

/// The part after `end_header`, contains the main data.
pub type Payload<E> = KeyMap<Vec<E>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ply::property::ScalarType;

    #[derive(Debug, PartialEq)]
    struct MockElement;

    impl PropertyAccess for MockElement {
        fn new() -> Self {
            MockElement
        }
    }

    #[test]
    fn test_mock_element_new() {
        let _ = MockElement::new();
    }

    #[test]
    fn test_version_display() {
        let v = Version { major: 1, minor: 0 };
        assert_eq!(format!("{}", v), "1.0");
        let v = Version { major: 1, minor: 1 };
        assert_eq!(format!("{}", v), "1.1");
    }

    #[test]
    fn test_encoding_display() {
        assert_eq!(format!("{}", Encoding::Ascii), "ascii");
        assert_eq!(
            format!("{}", Encoding::BinaryBigEndian),
            "binary_big_endian"
        );
        assert_eq!(
            format!("{}", Encoding::BinaryLittleEndian),
            "binary_little_endian"
        );
    }

    #[test]
    fn test_header_new() {
        let h = Header::new();
        assert_eq!(h.encoding, Encoding::Ascii);
        assert_eq!(h.version, Version { major: 1, minor: 0 });
        assert!(h.obj_infos.is_empty());
        assert!(h.elements.is_empty());
        assert!(h.comments.is_empty());
    }

    #[test]
    fn test_header_default() {
        assert_eq!(Header::default(), Header::new());
    }

    #[test]
    fn test_element_def_new() {
        let e = ElementDef::new("vertex".to_string());
        assert_eq!(e.name, "vertex");
        assert_eq!(e.count, 0);
        assert!(e.properties.is_empty());
    }

    #[test]
    fn test_property_def_new() {
        let pt = PropertyType::Scalar(ScalarType::Float);
        let p = PropertyDef::new("x".to_string(), pt.clone());
        assert_eq!(p.name, "x");
        assert_eq!(p.data_type, pt);
    }

    #[test]
    fn test_ply_new() {
        let ply = Ply::<MockElement>::new();
        assert_eq!(ply.header, Header::new());
        assert!(ply.payload.is_empty());
    }

    #[test]
    fn test_ply_default() {
        let ply = Ply::<MockElement>::default();
        assert_eq!(ply, Ply::<MockElement>::new());
    }
}