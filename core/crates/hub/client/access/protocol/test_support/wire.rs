use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fmt::Debug;

pub fn value(input: &impl Serialize) -> Value {
    serde_json::to_value(input).unwrap()
}

pub fn rejected<T: DeserializeOwned + Debug>(input: Value) {
    assert!(serde_json::from_value::<T>(input).is_err());
}

pub fn duplicate_field(input: &impl Serialize, field: &str) -> String {
    let object = value(input);
    let encoded = serde_json::to_string(&object).unwrap();
    let remainder = encoded.strip_prefix('{').unwrap();
    let key = serde_json::to_string(field).unwrap();
    let field_value = serde_json::to_string(&object[field]).unwrap();
    format!("{{{key}:{field_value},{remainder}")
}
