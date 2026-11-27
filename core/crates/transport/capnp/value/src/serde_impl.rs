use crate::Value;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Signed(value) => serializer.serialize_i64(*value),
            Self::Unsigned(value) => serializer.serialize_u64(*value),
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::Text(value) => serializer.serialize_str(value),
            Self::Data(value) => serializer.serialize_bytes(value),
            Self::List(values) => {
                let mut output = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    output.serialize_element(value)?;
                }
                output.end()
            }
            Self::Map(values) => {
                let mut output = serializer.serialize_map(Some(values.len()))?;
                let mut seen = std::collections::BTreeSet::new();
                for (key, value) in values {
                    let identity = serde_value::Value::try_from(key.clone())
                        .map_err(serde::ser::Error::custom)?;
                    if !seen.insert(identity) {
                        return Err(serde::ser::Error::custom(crate::ValueError::DuplicateKey));
                    }
                    output.serialize_entry(key, value)?;
                }
                output.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde_value::Value::deserialize(deserializer).map(Self::from)
    }
}
