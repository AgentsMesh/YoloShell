use super::*;
use yolo_capnp_value::ValueError;

#[test]
fn nested_duplicate_keys_cannot_disappear_inside_a_list_or_a_composite_key() {
    let duplicate = Value::Map(vec![
        (Value::from("writer"), Value::from(true)),
        (Value::from("writer"), Value::from(false)),
    ]);
    for tree in [
        Value::List(vec![duplicate.clone()]),
        Value::Map(vec![(duplicate.clone(), Value::Null)]),
        Value::Map(vec![(Value::from("grant"), duplicate)]),
    ] {
        assert!(matches!(
            from_value::<Value>(tree.clone()),
            Err(ValueError::DuplicateKey)
        ));
        assert!(to_value(&tree).is_err());
    }
}

#[test]
fn structural_key_identity_is_independent_of_map_order_and_signed_zero() {
    let fields = vec![
        (Value::from("a"), Value::from(1u64)),
        (Value::from("b"), Value::from(2u64)),
    ];
    let mut reverse = fields.clone();
    reverse.reverse();
    for (first, second) in [
        (Value::Map(fields), Value::Map(reverse)),
        (Value::Float(0.0), Value::Float(-0.0)),
        (Value::Float(f64::NAN), Value::Float(-f64::NAN)),
    ] {
        let tree = Value::Map(vec![
            (first, Value::from(true)),
            (second, Value::from(false)),
        ]);
        assert!(matches!(
            from_value::<Value>(tree.clone()),
            Err(ValueError::DuplicateKey)
        ));
        assert!(to_value(&tree).is_err());
    }
    let distinct = Value::Map(vec![
        (Value::from(1u64), Value::Data(vec![0])),
        (Value::from("1"), Value::Data(vec![u8::MAX])),
    ]);
    let decoded: Value = from_value(to_value(&distinct).unwrap()).unwrap();
    assert_eq!(
        decoded.as_map().unwrap().len(),
        distinct.as_map().unwrap().len()
    );
}
