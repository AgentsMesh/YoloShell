use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, Error, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;

struct Unique(Value);

/// Duplicate keys are rejected recursively, before any routing or side effect.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    let unique: Unique = serde_json::from_slice(bytes)?;
    serde_json::from_value(unique.0)
}

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON value with unique object keys")
    }
    fn visit_bool<E: Error>(self, value: bool) -> Result<Unique, E> {
        Ok(Unique(Value::Bool(value)))
    }
    fn visit_i64<E: Error>(self, value: i64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(value.into())))
    }
    fn visit_u64<E: Error>(self, value: u64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(value.into())))
    }
    fn visit_f64<E: Error>(self, value: f64) -> Result<Unique, E> {
        Number::from_f64(value)
            .map(|value| Unique(Value::Number(value)))
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<Unique, E> {
        Ok(Unique(Value::String(value.into())))
    }
    fn visit_string<E: Error>(self, value: String) -> Result<Unique, E> {
        Ok(Unique(Value::String(value)))
    }
    fn visit_none<E: Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
    fn visit_unit<E: Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Unique, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<Unique>()? {
            values.push(value.0);
        }
        Ok(Unique(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom("duplicate JSON object key"));
            }
            values.insert(key, map.next_value::<Unique>()?.0);
        }
        Ok(Unique(Value::Object(values)))
    }
}
