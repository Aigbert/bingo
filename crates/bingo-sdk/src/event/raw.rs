//! Capture JSON bytes, including when serde has already buffered a tagged enum.

use std::fmt;

use serde::de::{MapAccess, Visitor, value::MapDeserializer};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_json::value::RawValue;

pub(super) fn capture<'de, D: Deserializer<'de>>(d: D) -> Result<Box<RawValue>, D::Error> {
    Box::<RawValue>::deserialize(Buffered(d))
}

struct Buffered<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for Buffered<D> {
    type Error = D::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.0.deserialize_any(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        // RawValue supplies its own serde protocol name. JSON readers preserve
        // bytes; serde's buffered readers instead enter visit_newtype_struct.
        self.0
            .deserialize_newtype_struct(name, Fallback { name, visitor })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

struct Fallback<V> {
    name: &'static str,
    visitor: V,
}

impl<'de, V: Visitor<'de>> Visitor<'de> for Fallback<V> {
    type Value = V::Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.visitor.expecting(f)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        self.visitor.visit_map(map)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        let value = Value::deserialize(d)?;
        let json = serde_json::to_string(&value).map_err(serde::de::Error::custom)?;
        self.visitor
            .visit_map(MapDeserializer::new(std::iter::once((self.name, json))))
    }
}
