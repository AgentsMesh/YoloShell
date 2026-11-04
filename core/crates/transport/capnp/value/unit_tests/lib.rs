use serde::{Deserialize, Serialize};
use yolo_capnp_value::{Value, from_value, to_value};
mod boundaries;
mod structural_keys;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Example {
    id: u64,
    optional: Option<String>,
    #[serde(with = "serde_bytes")]
    bytes: Vec<u8>,
}

#[test]
fn owned_values_preserve_binary_optional_and_full_width_integers() {
    let original = Example {
        id: u64::MAX,
        optional: None,
        bytes: vec![0, 255, 27],
    };
    let value = to_value(&original).unwrap();
    let Value::Map(fields) = &value else { panic!() };
    assert!(
        fields
            .iter()
            .any(|(key, value)| key.as_str() == Some("bytes") && matches!(value, Value::Data(_)))
    );
    assert_eq!(from_value::<Example>(value).unwrap(), original);
}

#[test]
fn duplicate_fields_and_unknown_fields_are_rejected() {
    let value = to_value(&Example {
        id: 1,
        optional: None,
        bytes: vec![],
    })
    .unwrap();
    let Value::Map(mut fields) = value else {
        panic!()
    };
    fields.push((Value::from("id"), Value::Unsigned(2)));
    assert!(from_value::<Example>(Value::Map(fields.clone())).is_err());
    assert!(to_value(&Value::Map(fields.clone())).is_err());
    fields.pop();
    fields.push((Value::from("unexpected"), Value::Null));
    assert!(from_value::<Example>(Value::Map(fields)).is_err());
}

#[test]
fn enums_use_serde_domain_shapes() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Choice {
        Empty,
        Payload { value: i64 },
    }
    for choice in [Choice::Empty, Choice::Payload { value: i64::MIN }] {
        assert_eq!(
            from_value::<Choice>(to_value(&choice).unwrap()).unwrap(),
            choice
        );
    }
}
