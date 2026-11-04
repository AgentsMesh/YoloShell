use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_raw_and_folded_protocol_frame_widths() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/protocol.rs",
        "Vec::with_capacity(2 + 2 + payload.len());\n",
    );
    fixture.write(
        "core/crates/wire/src/framing.rs",
        "let length = [0_u8; 4];\n",
    );
    fixture.write(
        "integration_tests/runtime/protocol.rs",
        "decode(&frame[4..]);\n",
    );
    fixture.write(
        "core/crates/transport/remote/test_support/russh_backend/server_protocol.rs",
        "buffer.get(..2 + 2);\nbuffer.drain(..4);\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 6);
    assert_eq!(
        result
            .iter()
            .filter(|entry| entry.contains("FRAME_LENGTH_BYTES"))
            .count(),
        5
    );
    assert_eq!(
        result
            .iter()
            .filter(|entry| entry.contains("named constant"))
            .count(),
        1
    );
}

#[test]
fn permits_named_widths_and_unrelated_constants_arrays_and_capacities() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/wire/src/framing.rs",
        "const RETRY_LIMIT: usize = 4;\n\
         const VALUE_CAPACITY: usize = 4;\n\
         let scratch = [0_u8; 4];\n\
         let values = Vec::with_capacity(VALUE_CAPACITY);\n\
         let length = [0_u8; FRAME_LENGTH_BYTES];\n\
         let payload = &frame[FRAME_LENGTH_BYTES..];\n",
    );
    assert!(messages(&fixture).is_empty());
}
