use super::*;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WireWidths {
    version: u8,
    columns: u16,
    length: u32,
    sequence: u64,
    narrow_exit: i8,
    signed_columns: i16,
    exit: i32,
    timestamp: i64,
    scale: f32,
    ratio: f64,
    glyph: char,
    ready: bool,
    token: Token,
    nested: Option<Vec<Option<String>>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Token(u16);

const EXACT_FRACTION: f64 = -0.5;
const UTF8_TEXT: &str = "标题\0metadata";
const BINARY: &[u8] = b"\xff\0\n";

#[test]
fn typed_domain_widths_survive_structural_conversion_without_json_precision_loss() {
    let original = WireWidths {
        version: u8::MAX,
        columns: u16::MAX,
        length: u32::MAX,
        sequence: u64::MAX,
        narrow_exit: i8::MIN,
        signed_columns: i16::MIN,
        exit: i32::MIN,
        timestamp: i64::MIN,
        scale: EXACT_FRACTION as f32,
        ratio: EXACT_FRACTION,
        glyph: '你',
        ready: true,
        token: Token(u16::MAX),
        nested: Some(vec![None, Some(UTF8_TEXT.into())]),
    };
    let value = to_value(&original).unwrap();
    assert_eq!(from_value::<WireWidths>(value).unwrap(), original);
    for value in [
        Value::from(u8::MAX),
        Value::from(u16::MAX),
        Value::from(u32::MAX),
        Value::from(u64::MAX),
    ] {
        assert_eq!(
            from_value::<u64>(value.clone()).unwrap(),
            value.as_u64().unwrap()
        );
    }
    for value in [Value::from(i32::MIN), Value::from(i64::MIN)] {
        assert!(value.as_u64().is_none());
        assert!(from_value::<u64>(value).is_err());
    }
    assert_eq!(Value::from(0i64).as_u64(), Some(0));
    assert!(from_value::<u8>(Value::from(u16::MAX)).is_err());
    assert!(from_value::<i64>(Value::from(u64::MAX)).is_err());
}

#[test]
fn borrowed_domain_readers_do_not_coerce_binary_text_lists_or_numeric_strings() {
    let text = Value::from(UTF8_TEXT.to_owned());
    let bytes = Value::Data(BINARY.to_vec());
    let list = Value::List(vec![Value::from(true), Value::Null]);
    let map = Value::Map(vec![(Value::from("payload"), bytes.clone())]);
    assert_eq!(text.as_str(), Some(UTF8_TEXT));
    assert_eq!(bytes.as_slice(), Some(BINARY));
    assert_eq!(list.as_list().unwrap().len(), 2);
    assert_eq!(
        map.as_map().unwrap().first().unwrap().1.as_slice(),
        Some(BINARY)
    );
    assert_eq!(
        from_value::<serde_bytes::ByteBuf>(bytes.clone())
            .unwrap()
            .into_vec(),
        BINARY
    );
    assert_eq!(from_value::<Option<String>>(Value::Null).unwrap(), None);
    for value in [
        &text,
        &bytes,
        &list,
        &map,
        &Value::Null,
        &Value::from(false),
    ] {
        assert!(value.as_u64().is_none());
        if !matches!(value, Value::Map(_)) {
            assert!(value.as_map().is_none());
        }
        if !matches!(value, Value::List(_)) {
            assert!(value.as_list().is_none());
        }
        if !matches!(value, Value::Text(_)) {
            assert!(value.as_str().is_none());
        }
        if !matches!(value, Value::Data(_)) {
            assert!(value.as_slice().is_none());
        }
    }
    assert!(from_value::<u64>(Value::from("42")).is_err());
    assert!(from_value::<bool>(Value::from(1u64)).is_err());
    assert!(from_value::<String>(bytes).is_err());
}
