//! Owned structural values at the domain/transport boundary. Never a wire format.
mod conversion;
mod error;
mod serde_impl;

pub use error::ValueError;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    Text(String),
    Data(Vec<u8>),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
}

impl Value {
    pub fn as_map(&self) -> Option<&[(Value, Value)]> {
        match self {
            Self::Map(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Data(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Unsigned(value) => Some(*value),
            Self::Signed(value) => u64::try_from(*value).ok(),
            _ => None,
        }
    }
}

pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, ValueError> {
    serde_value::to_value(value)
        .map(Value::from)
        .map_err(|error| ValueError::Invalid(error.to_string()))
}

pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, ValueError> {
    let value = serde_value::Value::try_from(value)?;
    value
        .deserialize_into()
        .map_err(|error| ValueError::Invalid(error.to_string()))
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::Unsigned(value.into())
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Self::Unsigned(value.into())
    }
}
impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Unsigned(value.into())
    }
}
impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Signed(value.into())
    }
}
impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Signed(value)
    }
}
