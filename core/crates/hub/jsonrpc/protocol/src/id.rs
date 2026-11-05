use crate::MAX_ID_BYTES;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Transport correlation is deliberately independent from durable operation IDs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub struct Id(Value);

impl Id {
    pub fn fresh() -> Self {
        Self(Value::String(uuid::Uuid::new_v4().to_string()))
    }
}

impl TryFrom<Value> for Id {
    type Error = &'static str;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let valid = match &value {
            Value::String(text) => !text.is_empty() && text.len() <= MAX_ID_BYTES,
            Value::Number(number) => number.is_i64() || number.is_u64(),
            _ => false,
        };
        if valid {
            Ok(Self(value))
        } else {
            Err("invalid JSON-RPC correlation id")
        }
    }
}

impl From<Id> for Value {
    fn from(id: Id) -> Self {
        id.0
    }
}
