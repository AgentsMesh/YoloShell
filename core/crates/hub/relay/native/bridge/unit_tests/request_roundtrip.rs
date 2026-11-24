use crate::*;

fn request() -> types::Request {
    let id = |low| types::Identity { high: 1, low };
    types::Request {
        launch: id(1), hub: id(2), terminal: id(3), runtime: id(4), request: id(5),
        agent: id(6), execution: id(7), stream: id(8), sequence: u64::MAX,
        action: INTERRUPT, prompt: types::Text::default(),
    }
}

#[test]
fn prompt_bytes_and_full_width_cursor_survive_ffi_ownership_transfer() {
    let prompt = "hello\n世界".as_bytes();
    let mut request = request();
    request.action = SUBMIT;
    request.prompt = types::Text { data: prompt.as_ptr(), length: prompt.len() };
    let mut output = types::Buffer::default();
    assert_eq!(unsafe { yolo_native_relay_encode_request(&request, &mut output) }, OK);
    let mut decoded = types::Decoded::default();
    assert_eq!(unsafe { yolo_native_relay_decode(output.data, output.length, &mut decoded) }, OK);
    unsafe { yolo_native_relay_release_buffer(&mut output); }
    assert_eq!(decoded.kind, REQUEST);
    assert_eq!(decoded.request.sequence, u64::MAX);
    assert_eq!(decoded.request.action, SUBMIT);
    assert_eq!(unsafe { std::slice::from_raw_parts(decoded.prompt.data, decoded.prompt.length) }, prompt);
    unsafe { yolo_native_relay_release_decoded(&mut decoded); }
    assert!(decoded.request.prompt.data.is_null());
}

#[test]
fn all_receipt_and_not_sent_states_roundtrip() {
    let request = request();
    let outcomes = [
        types::Outcome::default(),
        types::Outcome { kind: RECEIPT, state: ACCEPTED, ..Default::default() },
        types::Outcome { kind: RECEIPT, state: DELIVERED, ..Default::default() },
        types::Outcome { kind: RECEIPT, state: CONFIRMED, ..Default::default() },
        types::Outcome { kind: RECEIPT, state: RECEIPT_UNKNOWN, ..Default::default() },
    ].into_iter()
        .chain([PERMISSION_DENIED, PRECONDITIONS_CHANGED, UNSUPPORTED, EXPIRED, ADAPTER_REJECTED]
            .map(|failure| types::Outcome { kind: RECEIPT, state: FAILED, failure, ..Default::default() }))
        .chain([NO_WRITER, NOT_SENT_EXPIRED, TARGET_CHANGED, UNAVAILABLE]
            .map(|reason| types::Outcome { kind: NOT_SENT, reason, ..Default::default() }));
    for outcome in outcomes {
        let mut output = types::Buffer::default();
        assert_eq!(unsafe { yolo_native_relay_encode_response(&request, &outcome, &mut output) }, OK);
        let mut decoded = types::Decoded::default();
        assert_eq!(unsafe { yolo_native_relay_decode(output.data, output.length, &mut decoded) }, OK);
        assert_eq!(decoded.kind, RESPONSE);
        assert_eq!((decoded.outcome.kind, decoded.outcome.state, decoded.outcome.failure, decoded.outcome.reason),
            (outcome.kind, outcome.state, outcome.failure, outcome.reason));
        unsafe { yolo_native_relay_release_decoded(&mut decoded); yolo_native_relay_release_buffer(&mut output); }
    }
}

#[test]
fn malformed_borrowed_requests_do_not_allocate_a_result() {
    let mut request = request();
    for field in [&mut request.launch, &mut request.hub, &mut request.terminal, &mut request.runtime,
        &mut request.request, &mut request.agent, &mut request.execution, &mut request.stream] {
        *field = types::Identity::default();
    }
    let mut output = types::Buffer::default();
    assert_eq!(unsafe { yolo_native_relay_encode_request(&request, &mut output) }, INVALID);
    assert!(output.data.is_null());
    let mut request = self::request();
    request.action = SUBMIT;
    request.prompt = types::Text { data: std::ptr::null(), length: 1 };
    assert_eq!(unsafe { yolo_native_relay_encode_request(&request, &mut output) }, INVALID);
    request.prompt = types::Text { data: b"\x1b".as_ptr(), length: 1 };
    assert_eq!(unsafe { yolo_native_relay_encode_request(&request, &mut output) }, INVALID);
    request.prompt.length = usize::MAX;
    assert_eq!(unsafe { yolo_native_relay_encode_request(&request, &mut output) }, INVALID);
}
