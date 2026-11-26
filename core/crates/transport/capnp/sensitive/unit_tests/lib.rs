use yolo_capnp_codec::Value;
use yolo_capnp_sensitive::{OwnedValue, encode, erase, read};
mod decode;

#[test]
fn sensitive_encoding_is_standard_capnp_and_scrubs_owned_nested_values() {
    let mut value = OwnedValue(Value::Map(vec![(
        Value::from("secret"),
        Value::List(vec![Value::from("private"), Value::Data(vec![1, 2, 3])]),
    )]));
    let bytes = encode(&value.0).unwrap();
    assert_eq!(yolo_capnp_codec::decode_value(&bytes).unwrap(), value.0);
    assert!(
        read(&bytes, |node| node
            .which()
            .map(|_| ())
            .map_err(|_| yolo_capnp_sensitive::invalid()))
        .is_ok()
    );
    erase(&mut value.0);
    assert_eq!(value.0, Value::Map(vec![]));
    assert!(read(b"private-json", |_| Ok(())).is_err());
}
