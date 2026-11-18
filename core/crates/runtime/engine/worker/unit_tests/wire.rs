use crate::worker::tests::*;
const INPUT_GENERATION: u64 = 7;
const RESIZE_GENERATION: u64 = 8;
const SIGNAL_GENERATION: u64 = 9;
const LEGACY_GENERATION: u64 = 6;
const WIRE_TEST_COLUMNS: u16 = 100;
const WIRE_TEST_ROWS: u16 = 40;
const TEST_INPUT_BYTE: u8 = 65;
const MALFORMED_RUNTIME_EPOCH: &str = "not-a-runtime-epoch";

#[test]
fn terminal_mutations_require_the_canonical_generation_field() {
    let terminal_id = Uuid::new_v4().to_string();
    let runtime_epoch = Uuid::new_v4().to_string();
    let input = serde_json::json!({
        (crate::protocol::field::TERMINAL_ID): terminal_id,
        (crate::protocol::field::RUNTIME_EPOCH): runtime_epoch,
        (crate::protocol::field::GENERATION): INPUT_GENERATION,
        (crate::protocol::field::WRITER_CAPABILITY): WriterCapability::generate(),
        (crate::protocol::field::INPUT_ID): Uuid::new_v4().to_string(),
        (crate::protocol::field::DATA): [TEST_INPUT_BYTE]
    });
    let decoded: InputRequest = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(decoded.generation, INPUT_GENERATION);

    let resize = serde_json::json!({
        (crate::protocol::field::TERMINAL_ID): terminal_id,
        (crate::protocol::field::RUNTIME_EPOCH): runtime_epoch,
        (crate::protocol::field::GENERATION): RESIZE_GENERATION,
        (crate::protocol::field::WRITER_CAPABILITY): WriterCapability::generate(),
        (crate::protocol::field::COLS): WIRE_TEST_COLUMNS,
        (crate::protocol::field::ROWS): WIRE_TEST_ROWS
    });
    let decoded: ResizeRequest = serde_json::from_value(resize.clone()).unwrap();
    assert_eq!(decoded.generation, RESIZE_GENERATION);

    let signal = serde_json::json!({
        (crate::protocol::field::TERMINAL_ID): terminal_id,
        (crate::protocol::field::RUNTIME_EPOCH): runtime_epoch,
        (crate::protocol::field::GENERATION): SIGNAL_GENERATION,
        (crate::protocol::field::WRITER_CAPABILITY): WriterCapability::generate(),
        (crate::protocol::field::SIGNAL): "CONT"
    });
    let decoded: SignalRequest = serde_json::from_value(signal.clone()).unwrap();
    assert_eq!(decoded.generation, SIGNAL_GENERATION);

    for mut value in [input, resize, signal] {
        let object = value.as_object_mut().unwrap();
        object.insert(
            crate::protocol::field::LEASE_GENERATION.into(),
            serde_json::json!(LEGACY_GENERATION),
        );
        let error = match object.contains_key(crate::protocol::field::INPUT_ID) {
            true => serde_json::from_value::<InputRequest>(value).unwrap_err(),
            false if object.contains_key(crate::protocol::field::COLS) => {
                serde_json::from_value::<ResizeRequest>(value).unwrap_err()
            }
            false => serde_json::from_value::<SignalRequest>(value).unwrap_err(),
        };
        assert!(
            error
                .to_string()
                .contains("unknown field `lease_generation`")
        );
    }
}

#[test]
fn terminal_mutation_wire_rejects_invalid_runtime_identity() {
    let mut input = serde_json::to_value(InputRequest {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        generation: INPUT_GENERATION,
        writer_capability: WriterCapability::generate(),
        input_id: Uuid::new_v4(),
        data: vec![TEST_INPUT_BYTE],
    })
    .unwrap();
    let valid_epoch = input[crate::protocol::field::RUNTIME_EPOCH].clone();
    for invalid in [
        None,
        Some(serde_json::Value::String(MALFORMED_RUNTIME_EPOCH.into())),
        Some(serde_json::Value::String(
            valid_epoch.as_str().unwrap().replace('-', ""),
        )),
        Some(serde_json::Value::String(Uuid::nil().to_string())),
    ] {
        let object = input.as_object_mut().unwrap();
        match invalid {
            Some(value) => {
                object.insert(crate::protocol::field::RUNTIME_EPOCH.into(), value);
            }
            None => {
                object.remove(crate::protocol::field::RUNTIME_EPOCH);
            }
        }
        assert!(serde_json::from_value::<InputRequest>(input.clone()).is_err());
    }
}
