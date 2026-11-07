use crate::{Value, ValueError};
use std::collections::BTreeMap;

impl From<serde_value::Value> for Value {
    fn from(value: serde_value::Value) -> Self {
        use serde_value::Value as Source;
        match value {
            Source::Bool(value) => Self::Boolean(value),
            Source::U8(value) => Self::Unsigned(value.into()),
            Source::U16(value) => Self::Unsigned(value.into()),
            Source::U32(value) => Self::Unsigned(value.into()),
            Source::U64(value) => Self::Unsigned(value),
            Source::I8(value) => Self::Signed(value.into()),
            Source::I16(value) => Self::Signed(value.into()),
            Source::I32(value) => Self::Signed(value.into()),
            Source::I64(value) => Self::Signed(value),
            Source::F32(value) => Self::Float(value.into()),
            Source::F64(value) => Self::Float(value),
            Source::Char(value) => Self::Text(value.to_string()),
            Source::String(value) => Self::Text(value),
            Source::Unit | Source::Option(None) => Self::Null,
            Source::Option(Some(value)) | Source::Newtype(value) => (*value).into(),
            Source::Seq(values) => Self::List(values.into_iter().map(Self::from).collect()),
            Source::Map(values) => Self::Map(
                values
                    .into_iter()
                    .map(|(key, value)| (key.into(), value.into()))
                    .collect(),
            ),
            Source::Bytes(value) => Self::Data(value),
        }
    }
}

impl TryFrom<Value> for serde_value::Value {
    type Error = ValueError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::Null => Self::Unit,
            Value::Boolean(value) => Self::Bool(value),
            Value::Signed(value) => Self::I64(value),
            Value::Unsigned(value) => Self::U64(value),
            Value::Float(value) => Self::F64(value),
            Value::Text(value) => Self::String(value),
            Value::Data(value) => Self::Bytes(value),
            Value::List(values) => Self::Seq(
                values
                    .into_iter()
                    .map(Self::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            Value::Map(values) => {
                let mut entries = BTreeMap::new();
                for (key, value) in values {
                    if entries.insert(key.try_into()?, value.try_into()?).is_some() {
                        return Err(ValueError::DuplicateKey);
                    }
                }
                Self::Map(entries)
            }
        })
    }
}
