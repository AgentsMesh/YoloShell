use yolo_capnp_codec::{
    MAX_VALUE_DEPTH, SCHEMA_VERSION, Value, ValueError, decode_value, encode_value,
};
use yolo_capnp_schema::value_capnp::{document, value};

const CHILD_COUNT: u32 = 1;
const CHILD_INDEX: u32 = 0;
const CHILD_DEPTH: usize = 1;
const MAP_KEY: &str = "nested";

#[test]
fn structural_depth_limit_is_independent_of_capnp_pointer_layout() {
    for maps in [false, true] {
        let mut nested = Value::Null;
        for _ in 0..MAX_VALUE_DEPTH {
            nested = if maps {
                Value::Map(vec![(Value::from(MAP_KEY), nested)])
            } else {
                Value::List(vec![nested])
            };
        }
        assert_eq!(
            decode_value(&encode_value(&nested).unwrap()).unwrap(),
            nested
        );
        assert!(matches!(
            encode_value(&Value::List(vec![nested])),
            Err(ValueError::Depth)
        ));
    }
}

#[test]
fn inbound_pointer_budget_does_not_weaken_the_structural_depth_limit() {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<document::Builder<'_>>();
    root.set_version(SCHEMA_VERSION);
    raw_nested(root.init_value(), MAX_VALUE_DEPTH + CHILD_DEPTH);
    let bytes = capnp::serialize::write_message_to_words(&builder);
    assert!(matches!(decode_value(&bytes), Err(ValueError::Depth)));
}

fn raw_nested(mut builder: value::Builder<'_>, remaining: usize) {
    if remaining == usize::default() {
        builder.set_null(());
    } else {
        raw_nested(
            builder.init_list(CHILD_COUNT).get(CHILD_INDEX),
            remaining - CHILD_DEPTH,
        );
    }
}
