//! Default element representation.
//!
//! [`DefaultElement`] is the ready-to-use payload element type provided by this crate.
//! It is a map from property name to [`Property`], preserving insertion order.

use super::KeyMap;
use super::Property;
use super::PropertyAccess;
use std::borrow::Cow;

/// Ready to use data-structure for all kind of element definitions.
///
/// PLY files carry the payload format in their header section.
/// Hence, they can contain all kinds of elements, or formulated differently,
/// they define types very dynamically.
/// To achieve this flexibility in rust, this alias to a HashMap is provided.
///
/// If you need a more compact representation or faster access,
/// you might want to define your own structures and implement the `PropertyAccess` trait.
pub type DefaultElement = KeyMap<Property>;
macro_rules! get(
    ($e:expr) => (match $e {None => return None, Some(x) => x})
);
impl PropertyAccess for DefaultElement {
    fn new() -> Self {
        DefaultElement::new()
    }
    fn set_property(&mut self, key: &str, property: Property) {
        self.insert(key.to_string(), property);
    }
    fn get_char(&self, key: &str) -> Option<i8> {
        match *get!(self.get(key)) {
            Property::Char(x) => Some(x),
            _ => None,
        }
    }
    fn get_uchar(&self, key: &str) -> Option<u8> {
        match *get!(self.get(key)) {
            Property::UChar(x) => Some(x),
            _ => None,
        }
    }
    fn get_short(&self, key: &str) -> Option<i16> {
        match *get!(self.get(key)) {
            Property::Short(x) => Some(x),
            _ => None,
        }
    }
    fn get_ushort(&self, key: &str) -> Option<u16> {
        match *get!(self.get(key)) {
            Property::UShort(x) => Some(x),
            _ => None,
        }
    }
    fn get_int(&self, key: &str) -> Option<i32> {
        match *get!(self.get(key)) {
            Property::Int(x) => Some(x),
            _ => None,
        }
    }
    fn get_uint(&self, key: &str) -> Option<u32> {
        match *get!(self.get(key)) {
            Property::UInt(x) => Some(x),
            _ => None,
        }
    }
    fn get_float(&self, key: &str) -> Option<f32> {
        match *get!(self.get(key)) {
            Property::Float(x) => Some(x),
            _ => None,
        }
    }
    fn get_double(&self, key: &str) -> Option<f64> {
        match *get!(self.get(key)) {
            Property::Double(x) => Some(x),
            _ => None,
        }
    }
    fn get_list_char(&self, key: &str) -> Option<Cow<'_, [i8]>> {
        match *get!(self.get(key)) {
            Property::ListChar(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_uchar(&self, key: &str) -> Option<Cow<'_, [u8]>> {
        match *get!(self.get(key)) {
            Property::ListUChar(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_short(&self, key: &str) -> Option<Cow<'_, [i16]>> {
        match *get!(self.get(key)) {
            Property::ListShort(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_ushort(&self, key: &str) -> Option<Cow<'_, [u16]>> {
        match *get!(self.get(key)) {
            Property::ListUShort(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_int(&self, key: &str) -> Option<Cow<'_, [i32]>> {
        match *get!(self.get(key)) {
            Property::ListInt(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_uint(&self, key: &str) -> Option<Cow<'_, [u32]>> {
        match *get!(self.get(key)) {
            Property::ListUInt(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_float(&self, key: &str) -> Option<Cow<'_, [f32]>> {
        match *get!(self.get(key)) {
            Property::ListFloat(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
    fn get_list_double(&self, key: &str) -> Option<Cow<'_, [f64]>> {
        match *get!(self.get(key)) {
            Property::ListDouble(ref x) => Some(Cow::Borrowed(x)),
            _ => None,
        }
    }
}
