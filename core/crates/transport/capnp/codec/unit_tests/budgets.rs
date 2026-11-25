use yolo_capnp_codec::{
    MAX_MESSAGE_BYTES, Value, ValueError, decode_value, encode_value, read_value, write_value,
};
use yolo_capnp_schema::value_capnp::value;

const VALID_HEADROOM_BYTES: usize = 64 * 1024;
const ITEM_ALLOCATION_BYTES: usize = 256;
const ITEM_PAYLOAD_BYTES: usize = 240;
const ITEM_COUNT: usize = MAX_MESSAGE_BYTES / ITEM_ALLOCATION_BYTES;
const PAYLOAD_BYTE: u8 = b'x';

#[test]
fn rpc_value_writes_and_reads_reserve_message_overhead() {
    let data = vec![PAYLOAD_BYTE; MAX_MESSAGE_BYTES];
    let mut outbound = capnp::message::Builder::new_default();
    assert!(matches!(
        write_value(
            outbound.init_root::<value::Builder<'_>>(),
            &Value::Data(data.clone())
        ),
        Err(ValueError::Limit)
    ));
    let mut incoming = capnp::message::Builder::new_default();
    incoming.init_root::<value::Builder<'_>>().set_data(&data);
    assert!(matches!(
        read_value(incoming.get_root_as_reader::<value::Reader<'_>>().unwrap()),
        Err(ValueError::Limit)
    ));
}

#[test]
fn aggregate_struct_and_padding_costs_cannot_bypass_the_rpc_limit() {
    let text = String::from_utf8(vec![PAYLOAD_BYTE; ITEM_PAYLOAD_BYTES]).unwrap();
    let source = Value::List(vec![Value::Text(text.clone()); ITEM_COUNT]);
    let mut outbound = capnp::message::Builder::new_default();
    assert!(matches!(
        write_value(outbound.init_root::<value::Builder<'_>>(), &source),
        Err(ValueError::Limit)
    ));
    // Model a remote writer bypassing our encoder; the reader applies the same budget.
    let mut incoming = capnp::message::Builder::new_default();
    {
        let mut list = incoming
            .init_root::<value::Builder<'_>>()
            .init_list(ITEM_COUNT as u32);
        for index in 0..ITEM_COUNT {
            list.reborrow().get(index as u32).set_text(text.as_str());
        }
    }
    assert!(matches!(
        read_value(incoming.get_root_as_reader::<value::Reader<'_>>().unwrap()),
        Err(ValueError::Limit)
    ));
}

#[test]
fn large_valid_documents_roundtrip_without_double_charging_traversal() {
    let source = Value::Data(vec![PAYLOAD_BYTE; MAX_MESSAGE_BYTES - VALID_HEADROOM_BYTES]);
    let bytes = encode_value(&source).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode_value(&bytes).unwrap(), source);
}
