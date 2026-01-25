//! Map and helper traits used throughout the crate.
//!
//! The PLY format is inherently dynamic (elements/properties are declared in the
//! header). This module provides the [`KeyMap`] alias (currently backed by
//! [`indexmap::IndexMap`]) and small helper traits used to keep names consistent.

use indexmap::IndexMap;
use super::ElementDef;
use super::PropertyDef;

/// Alias to reduce coupling with map implementation
pub type KeyMap<V> = IndexMap<String, V>;

/// Convenience trait to assure consistency between map key and name attribute of stored element.
pub trait Addable<V: Key> {
    /// Takes a value that provides a key and stores it under the given key.
    fn add(&mut self, new_value: V);
}


impl<V: Key> Addable<V> for KeyMap<V> {
    fn add(&mut self, value: V) {
        self.insert(value.get_key(), value);
    }
}

/// Convenience trait to assure consistency between the key used for storage and the name of the elment.
pub trait Key {
    /// Returns a key under which the element should be stored in a key-value store.
    fn get_key(&self) -> String;
}
impl Key for ElementDef {
    fn get_key(&self) -> String {
        self.name.clone()
    }
}

impl Key for PropertyDef {
    fn get_key(&self) -> String {
        self.name.clone()
    }
}
