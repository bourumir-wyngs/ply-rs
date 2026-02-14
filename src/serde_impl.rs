use std::io::{Read, BufRead, BufReader, Write};
use serde::{Deserializer, Serialize, de, ser};
use crate::parser;
use crate::writer;
use crate::ply::{Header, Property, DefaultElement, Encoding, Ply, ElementDef, PropertyDef, PropertyType, ScalarType, Addable};
use crate::errors::{PlyResult, PlyError};

// ============================================================================
// Deserialization
// ============================================================================

/// Deserialize a PLY file into a struct.
pub fn from_reader<R, T>(r: R) -> PlyResult<T>
where
    R: Read,
    T: de::DeserializeOwned,
{
    let mut deserializer = PlyDeserializer::from_reader(r)?;
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

struct PlyDeserializer<R: Read> {
    parser: parser::Parser<DefaultElement>,
    reader: BufReader<R>, // Wrap in BufReader to support lines
    header: Header,
    current_element_idx: usize,
}

impl<R: Read> PlyDeserializer<R> {
    fn from_reader(r: R) -> PlyResult<Self> {
        let parser = parser::Parser::<DefaultElement>::new();
        let mut reader = BufReader::new(r);
        let header = parser.read_header(&mut reader)?;
        Ok(PlyDeserializer {
            parser,
            reader,
            header,
            current_element_idx: 0,
        })
    }
}

impl<'de, 'a, R: Read> Deserializer<'de> for &'a mut PlyDeserializer<R> {
    type Error = PlyError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(PlyMapAccess::new(self))
    }

    // Forward other methods to map or error
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct enum identifier ignored_any
    }
}

struct PlyMapAccess<'a, R: Read> {
    de: &'a mut PlyDeserializer<R>,
}

impl<'a, R: Read> PlyMapAccess<'a, R> {
    fn new(de: &'a mut PlyDeserializer<R>) -> Self {
        PlyMapAccess { de }
    }
}

impl<'de, 'a, R: Read> de::MapAccess<'de> for PlyMapAccess<'a, R> {
    type Error = PlyError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.de.current_element_idx >= self.de.header.elements.len() {
            return Ok(None);
        }
        let element_name = self.de.header.elements.keys().nth(self.de.current_element_idx).unwrap().clone();
        seed.deserialize(de::IntoDeserializer::into_deserializer(element_name))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let element_name = self.de.header.elements.keys().nth(self.de.current_element_idx).unwrap().clone();
        let element_def = self.de.header.elements.get(&element_name).unwrap().clone();
        
        self.de.current_element_idx += 1;

        let seq_access = PlyElementSeqAccess {
            de: self.de,
            element_def,
            current_count: 0,
        };

        seed.deserialize(SeqDeserializer(seq_access))
    }
}

struct SeqDeserializer<'a, R: Read>(PlyElementSeqAccess<'a, R>);

impl<'de, 'a, R: Read> Deserializer<'de> for SeqDeserializer<'a, R> {
    type Error = PlyError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        visitor.visit_seq(self.0)
    }
    
    // Forward all to deserialize_any
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct PlyElementSeqAccess<'a, R: Read> {
    de: &'a mut PlyDeserializer<R>,
    element_def: ElementDef,
    current_count: usize,
}

impl<'de, 'a, R: Read> de::SeqAccess<'de> for PlyElementSeqAccess<'a, R> {
    type Error = PlyError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.current_count >= self.element_def.count {
            return Ok(None);
        }

        self.current_count += 1;

        let element = match self.de.header.encoding {
            Encoding::Ascii => {
                let mut line = String::new();
                self.de.reader.read_line(&mut line).map_err(PlyError::Io)?;
                self.de.parser.read_ascii_element(&line, &self.element_def)?
            },
            Encoding::BinaryBigEndian => {
                self.de.parser.read_big_endian_element(&mut self.de.reader, &self.element_def)?
            },
            Encoding::BinaryLittleEndian => {
                self.de.parser.read_little_endian_element(&mut self.de.reader, &self.element_def)?
            },
        };

        let element_deserializer = ElementDeserializer {
            element,
            element_def: &self.element_def,
        };
        
        seed.deserialize(element_deserializer).map(Some)
    }
}

struct ElementDeserializer<'b> {
    element: DefaultElement,
    element_def: &'b ElementDef,
}

impl<'de, 'b> Deserializer<'de> for ElementDeserializer<'b> {
    type Error = PlyError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        self.deserialize_map(visitor)
    }
    

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
         visitor.visit_map(ElementPropertyAccess {
             element: self.element,
             element_def: self.element_def,
             current_prop_idx: 0,
         })
    }
    
    // Boilerplate forwarding...
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct struct enum identifier ignored_any
    }
}

struct ElementPropertyAccess<'b> {
    element: DefaultElement,
    element_def: &'b ElementDef,
    current_prop_idx: usize,
}

impl<'de, 'b> de::MapAccess<'de> for ElementPropertyAccess<'b> {
    type Error = PlyError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.current_prop_idx >= self.element_def.properties.len() {
            return Ok(None);
        }
        
        let prop_name = self.element_def.properties.keys().nth(self.current_prop_idx).unwrap().clone();
        seed.deserialize(de::IntoDeserializer::into_deserializer(prop_name)).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let prop_name = self.element_def.properties.keys().nth(self.current_prop_idx).unwrap().clone();
        let prop_value = self.element.get(&prop_name).unwrap();
        
        self.current_prop_idx += 1;
        
        seed.deserialize(PropertyDeserializer(prop_value))
    }
}

struct PropertyDeserializer<'a>(&'a Property);

impl<'de, 'a> Deserializer<'de> for PropertyDeserializer<'a> {
    type Error = PlyError;
    
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        match self.0 {
            Property::Char(v) => visitor.visit_i8(*v),
            Property::UChar(v) => visitor.visit_u8(*v),
            Property::Short(v) => visitor.visit_i16(*v),
            Property::UShort(v) => visitor.visit_u16(*v),
            Property::Int(v) => visitor.visit_i32(*v),
            Property::UInt(v) => visitor.visit_u32(*v),
            Property::Float(v) => visitor.visit_f32(*v),
            Property::Double(v) => visitor.visit_f64(*v),
            Property::ListChar(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListUChar(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListShort(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListUShort(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListInt(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListUInt(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListFloat(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
            Property::ListDouble(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.iter().cloned())),
        }
    }
    
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        self.deserialize_any(visitor)
    }

    // Forward hints to deserialize_any
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        match self.0 {
            Property::Double(v) => visitor.visit_i64(v.round() as i64),
            Property::Float(v) => visitor.visit_i64(v.round() as i64),
            _ => self.deserialize_any(visitor),
        }
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        match self.0 {
            Property::Double(v) => visitor.visit_i128(v.round() as i128),
            Property::Float(v) => visitor.visit_i128(v.round() as i128),
            _ => self.deserialize_any(visitor),
        }
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        match self.0 {
            Property::Double(v) => visitor.visit_u64(v.round() as u64),
            Property::Float(v) => visitor.visit_u64(v.round() as u64),
            _ => self.deserialize_any(visitor),
        }
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
        match self.0 {
            Property::Double(v) => visitor.visit_u128(v.round() as u128),
            Property::Float(v) => visitor.visit_u128(v.round() as u128),
            _ => self.deserialize_any(visitor),
        }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> { self.deserialize_any(visitor) }
}

// ============================================================================
// Serialization
// ============================================================================

/// Serialize a struct to a PLY file.
pub fn to_writer<W, T>(w: W, value: &T) -> PlyResult<()>
where
    W: Write,
    T: Serialize,
{
    let mut serializer = PlySerializer::new();
    value.serialize(&mut serializer)?;
    
    // Construct header from collected data
    let mut ply = serializer.ply;
    
    // We need to infer header if it's empty.
    // The serializer populates ply.payload.
    // ply.header should be constructed based on payload keys and first element of each list.
    
    // If header is empty (which it is by default), we infer.
    if ply.header.elements.is_empty() {
        for (name, list) in &ply.payload {
            let count = list.len();
            let mut elem_def = ElementDef::new(name.clone());
            elem_def.count = count;
            
            if let Some(first) = list.first() {
                // Infer properties from the first element
                for (prop_name, prop_val) in first {
                    let type_def = match prop_val {
                        Property::Char(_) => PropertyType::Scalar(ScalarType::Char),
                        Property::UChar(_) => PropertyType::Scalar(ScalarType::UChar),
                        Property::Short(_) => PropertyType::Scalar(ScalarType::Short),
                        Property::UShort(_) => PropertyType::Scalar(ScalarType::UShort),
                        Property::Int(_) => PropertyType::Scalar(ScalarType::Int),
                        Property::UInt(_) => PropertyType::Scalar(ScalarType::UInt),
                        Property::Float(_) => PropertyType::Scalar(ScalarType::Float),
                        Property::Double(_) => PropertyType::Scalar(ScalarType::Double),
                        Property::ListChar(_) => PropertyType::List(ScalarType::UChar, ScalarType::Char), // Lists usually store length as uchar or int
                        Property::ListUChar(_) => PropertyType::List(ScalarType::UChar, ScalarType::UChar),
                        Property::ListShort(_) => PropertyType::List(ScalarType::UChar, ScalarType::Short),
                        Property::ListUShort(_) => PropertyType::List(ScalarType::UChar, ScalarType::UShort),
                        Property::ListInt(_) => PropertyType::List(ScalarType::UChar, ScalarType::Int),
                        Property::ListUInt(_) => PropertyType::List(ScalarType::UChar, ScalarType::UInt),
                        Property::ListFloat(_) => PropertyType::List(ScalarType::UChar, ScalarType::Float),
                        Property::ListDouble(_) => PropertyType::List(ScalarType::UChar, ScalarType::Double),
                    };
                    elem_def.properties.add(PropertyDef::new(prop_name.clone(), type_def));
                }
            }
            ply.header.elements.add(elem_def);
        }
    }

    let writer = writer::Writer::new();
    let mut w = w;
    writer.write_ply(&mut w, &mut ply)?;
    Ok(())
}

struct PlySerializer {
    ply: Ply<DefaultElement>,
}

impl PlySerializer {
    fn new() -> Self {
        PlySerializer {
            ply: Ply::new(),
        }
    }
}

impl<'a> ser::Serializer for &'a mut PlySerializer {
    type Ok = ();
    type Error = PlyError;

    type SerializeSeq = ser::Impossible<(), PlyError>;
    type SerializeTuple = ser::Impossible<(), PlyError>;
    type SerializeTupleStruct = ser::Impossible<(), PlyError>;
    type SerializeTupleVariant = ser::Impossible<(), PlyError>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<(), PlyError>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(PlyError::Serialize("Top-level bool not supported".into()))
    }
    // ... forward scalars to error ...
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> { err_top_level_scalar() }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> { Ok(()) }
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> { Ok(()) }
    
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> {
         Err(PlyError::Serialize("Top-level newtype variant not supported".into()))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(PlyError::Serialize("Top-level sequence not supported. Use a struct or map with elements.".into()))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(PlyError::Serialize("Top-level tuple not supported".into()))
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(PlyError::Serialize("Top-level tuple struct not supported".into()))
    }

    fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(PlyError::Serialize("Top-level tuple variant not supported".into()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer { ply: &mut self.ply })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer { ply: &mut self.ply })
    }

    fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(PlyError::Serialize("Top-level struct variant not supported".into()))
    }
}

struct MapSerializer<'a> {
    ply: &'a mut Ply<DefaultElement>,
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, _key: &T) -> Result<(), Self::Error> {
        // We only support string keys for elements
        // But SerializeMap calls serialize_key then serialize_value.
        // We need to capture the key to use it in serialize_value.
        // But this trait doesn't allow state passing easily between key and value except via self.
        // So we need to store the key in self.
        Err(PlyError::Serialize("Map keys must be strings".into()))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> {
        Err(PlyError::Serialize("Map value called without key".into()))
    }

    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(&mut self, key: &K, value: &V) -> Result<(), Self::Error> {
        // HACK: Serialize key to string
        let key_str = KeySerializer::serialize_key(key)?;
        
        // Value must be a sequence of elements
        let mut element_list_serializer = ElementListSerializer { elements: Vec::new() };
        value.serialize(&mut element_list_serializer)?;
        
        self.ply.payload.insert(key_str, element_list_serializer.elements);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct StructSerializer<'a> {
    ply: &'a mut Ply<DefaultElement>,
}

impl<'a> ser::SerializeStruct for StructSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        // Value must be a sequence of elements
        let mut element_list_serializer = ElementListSerializer { elements: Vec::new() };
        value.serialize(&mut element_list_serializer)?;
        
        self.ply.payload.insert(key.to_string(), element_list_serializer.elements);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct KeySerializer;
impl KeySerializer {
    fn serialize_key<T: ?Sized + Serialize>(key: &T) -> Result<String, PlyError> {
        // Use a simple serializer to capture string
        struct StringCapturer(String);
        impl ser::Serializer for &mut StringCapturer {
            type Ok = ();
            type Error = PlyError;
            type SerializeSeq = ser::Impossible<(), PlyError>;
            type SerializeTuple = ser::Impossible<(), PlyError>;
            type SerializeTupleStruct = ser::Impossible<(), PlyError>;
            type SerializeTupleVariant = ser::Impossible<(), PlyError>;
            type SerializeMap = ser::Impossible<(), PlyError>;
            type SerializeStruct = ser::Impossible<(), PlyError>;
            type SerializeStructVariant = ser::Impossible<(), PlyError>;
            fn serialize_bool(self, _v: bool) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_i8(self, _v: i8) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_i16(self, _v: i16) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_i32(self, _v: i32) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_i64(self, _v: i64) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_u8(self, _v: u8) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_u16(self, _v: u16) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_u32(self, _v: u32) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_u64(self, _v: u64) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_f32(self, _v: f32) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_f64(self, _v: f64) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_char(self, _v: char) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_str(self, v: &str) -> Result<(), PlyError> { self.0 = v.to_string(); Ok(()) }
            fn serialize_bytes(self, _v: &[u8]) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_none(self) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), PlyError> { value.serialize(self) }
            fn serialize_unit(self) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_unit_struct(self, _name: &'static str) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<(), PlyError> { key_must_be_string() }
            fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<(), PlyError> { value.serialize(self) }
            fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, value: &T) -> Result<(), PlyError> { value.serialize(self) }
            fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, PlyError> { key_must_be_string() }
            fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, PlyError> { key_must_be_string() }
            fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, PlyError> { key_must_be_string() }
            fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, PlyError> { key_must_be_string() }
            fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, PlyError> { key_must_be_string() }
            fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, PlyError> { key_must_be_string() }
            fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, PlyError> { key_must_be_string() }
        }
        let mut capturer = StringCapturer(String::new());
        key.serialize(&mut capturer)?;
        Ok(capturer.0)
    }
}

struct ElementListSerializer {
    elements: Vec<DefaultElement>,
}

impl<'a> ser::Serializer for &'a mut ElementListSerializer {
    type Ok = ();
    type Error = PlyError;
    type SerializeSeq = ElementListSeqSerializer<'a>;
    type SerializeTuple = ser::Impossible<(), PlyError>;
    type SerializeTupleStruct = ser::Impossible<(), PlyError>;
    type SerializeTupleVariant = ser::Impossible<(), PlyError>;
    type SerializeMap = ser::Impossible<(), PlyError>;
    type SerializeStruct = ser::Impossible<(), PlyError>;
    type SerializeStructVariant = ser::Impossible<(), PlyError>;
    
    // ... all other methods error ...
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> { err_expected_sequence() }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> {
         Err(PlyError::Serialize("Newtype variant in element list not supported".into()))
    }
    
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ElementListSeqSerializer { list: self })
    }
    
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> { err_expected_sequence() }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> { err_expected_sequence() }
    fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> { err_expected_sequence() }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> { err_expected_sequence() }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> { err_expected_sequence() }
    fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> { err_expected_sequence() }
}

struct ElementListSeqSerializer<'a> {
    list: &'a mut ElementListSerializer,
}

impl<'a> ser::SerializeSeq for ElementListSeqSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let mut element_serializer = ElementSerializer { element: DefaultElement::new() };
        value.serialize(&mut element_serializer)?;
        self.list.elements.push(element_serializer.element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct ElementSerializer {
    element: DefaultElement,
}

fn err_must_be_struct_or_map<T>() -> Result<T, PlyError> {
    Err(PlyError::Serialize("Element must be a struct or map".into()))
}

fn key_must_be_string<T>() -> Result<T, PlyError> {
    Err(PlyError::Serialize("Key must be string".into()))
}

fn err_top_level_scalar<T>() -> Result<T, PlyError> {
    Err(PlyError::Serialize("Top-level scalar not supported".into()))
}

fn err_expected_sequence<T>() -> Result<T, PlyError> {
    Err(PlyError::Serialize("Expected sequence of elements".into()))
}

impl<'a> ser::Serializer for &'a mut ElementSerializer {
    
    type Ok = ();
    type Error = PlyError;
    type SerializeSeq = ser::Impossible<(), PlyError>;
    type SerializeTuple = ser::Impossible<(), PlyError>;
    type SerializeTupleStruct = ser::Impossible<(), PlyError>;
    type SerializeTupleVariant = ser::Impossible<(), PlyError>;
    type SerializeMap = ElementMapSerializer<'a>;
    type SerializeStruct = ElementStructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<(), PlyError>;

    // ... scalars error ...
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    // ... implement others as error ...
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> {
         Err(PlyError::Serialize("Newtype variant in element not supported".into()))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> { err_must_be_struct_or_map() }
    fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> { err_must_be_struct_or_map() }
    
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ElementMapSerializer { element: &mut self.element })
    }
    
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ElementStructSerializer { element: &mut self.element })
    }
    
    fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> { err_must_be_struct_or_map() }
}

struct ElementMapSerializer<'a> {
    element: &'a mut DefaultElement,
}

impl<'a> ser::SerializeMap for ElementMapSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, _key: &T) -> Result<(), Self::Error> { Err(PlyError::Serialize("Element map requires string keys".into())) }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Err(PlyError::Serialize("Element map value called without key".into())) }

    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(&mut self, key: &K, value: &V) -> Result<(), Self::Error> {
        let key_str = KeySerializer::serialize_key(key)?;
        let mut prop_serializer = PropertySerializer { property: None };
        value.serialize(&mut prop_serializer)?;
        if let Some(prop) = prop_serializer.property {
            self.element.insert(key_str, prop);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
}

struct ElementStructSerializer<'a> {
    element: &'a mut DefaultElement,
}

impl<'a> ser::SerializeStruct for ElementStructSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        let mut prop_serializer = PropertySerializer { property: None };
        value.serialize(&mut prop_serializer)?;
        if let Some(prop) = prop_serializer.property {
            self.element.insert(key.to_string(), prop);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
}

struct PropertySerializer {
    property: Option<Property>,
}

impl<'a> ser::Serializer for &'a mut PropertySerializer {
    type Ok = ();
    type Error = PlyError;
    type SerializeSeq = PropertySeqSerializer<'a>;
    // ... others impossible ...
    type SerializeTuple = ser::Impossible<(), PlyError>;
    type SerializeTupleStruct = ser::Impossible<(), PlyError>;
    type SerializeTupleVariant = ser::Impossible<(), PlyError>;
    type SerializeMap = ser::Impossible<(), PlyError>;
    type SerializeStruct = ser::Impossible<(), PlyError>;
    type SerializeStructVariant = ser::Impossible<(), PlyError>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> { Err(PlyError::Serialize("Boolean properties not supported".into())) }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::Char(v)); Ok(()) }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::Short(v)); Ok(()) }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::Int(v)); Ok(()) }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.property = Some(Property::Double(v as f64));
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.property = Some(Property::Double(v as f64));
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::UChar(v)); Ok(()) }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::UShort(v)); Ok(()) }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::UInt(v)); Ok(()) }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.property = Some(Property::Double(v as f64));
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.property = Some(Property::Double(v as f64));
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::Float(v)); Ok(()) }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> { self.property = Some(Property::Double(v)); Ok(()) }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> { 
        // Cast to i8
        self.property = Some(Property::Char(v as i8)); 
        Ok(()) 
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> { 
        // String -> ListChar
        let chars: Vec<i8> = v.bytes().map(|b| b as i8).collect();
        self.property = Some(Property::ListChar(chars));
        Ok(())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> { Err(PlyError::Serialize("Bytes not supported".into())) }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> { self.property = None; Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { Err(PlyError::Serialize("Unit not supported".into())) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> { Err(PlyError::Serialize("Unit struct not supported".into())) }
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> { Err(PlyError::Serialize("Unit variant not supported".into())) }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> { value.serialize(self) }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> {
         Err(PlyError::Serialize("Newtype variant in property not supported".into()))
    }
    
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(PropertySeqSerializer { property_serializer: self, list: Vec::new() })
    }
    
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> { Err(PlyError::Serialize("Tuple not supported".into())) }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> { Err(PlyError::Serialize("Tuple struct not supported".into())) }
    fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> { Err(PlyError::Serialize("Tuple variant not supported".into())) }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> { Err(PlyError::Serialize("Map not supported in property".into())) }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> { Err(PlyError::Serialize("Struct not supported in property".into())) }
    fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> { Err(PlyError::Serialize("Struct variant not supported".into())) }
}

struct PropertySeqSerializer<'a> {
    property_serializer: &'a mut PropertySerializer,
    list: Vec<Property>,
}

impl<'a> ser::SerializeSeq for PropertySeqSerializer<'a> {
    type Ok = ();
    type Error = PlyError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let mut ps = PropertySerializer { property: None };
        value.serialize(&mut ps)?;
        if let Some(prop) = ps.property {
            self.list.push(prop);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // Check homogeneity
        if self.list.is_empty() {
             // We can't determine type. Default to ListInt?
             self.property_serializer.property = Some(Property::ListInt(Vec::new()));
             return Ok(());
        }
        
        // Coerce all to the same list type based on first element
        // Since Property stores the value, we need to extract values.
        let first = &self.list[0];
        match first {
            Property::Char(_) => {
                let vec: Result<Vec<i8>, _> = self.list.into_iter().map(|p| match p { Property::Char(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListChar(v)); return Ok(()); }
            },
            Property::UChar(_) => {
                let vec: Result<Vec<u8>, _> = self.list.into_iter().map(|p| match p { Property::UChar(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListUChar(v)); return Ok(()); }
            },
            Property::Short(_) => {
                let vec: Result<Vec<i16>, _> = self.list.into_iter().map(|p| match p { Property::Short(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListShort(v)); return Ok(()); }
            },
            Property::UShort(_) => {
                let vec: Result<Vec<u16>, _> = self.list.into_iter().map(|p| match p { Property::UShort(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListUShort(v)); return Ok(()); }
            },
            Property::Int(_) => {
                let vec: Result<Vec<i32>, _> = self.list.into_iter().map(|p| match p { Property::Int(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListInt(v)); return Ok(()); }
            },
            Property::UInt(_) => {
                let vec: Result<Vec<u32>, _> = self.list.into_iter().map(|p| match p { Property::UInt(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListUInt(v)); return Ok(()); }
            },
            Property::Float(_) => {
                let vec: Result<Vec<f32>, _> = self.list.into_iter().map(|p| match p { Property::Float(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListFloat(v)); return Ok(()); }
            },
            Property::Double(_) => {
                let vec: Result<Vec<f64>, _> = self.list.into_iter().map(|p| match p { Property::Double(v) => Ok(v), _ => Err(()) }).collect();
                if let Ok(v) = vec { self.property_serializer.property = Some(Property::ListDouble(v)); return Ok(()); }
            },
            _ => return Err(PlyError::Serialize("Nested lists not supported".into())),
        }
        
        Err(PlyError::Serialize("Heterogeneous lists not supported".into()))
    }
}
