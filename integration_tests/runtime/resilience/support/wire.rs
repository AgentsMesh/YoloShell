use super::*;

const WIRE_RESPONSE_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;

pub(crate) async fn read_until(mut attachment: yolo_client::Attachment, needle: &[u8]) -> Vec<u8> {
    let mut output = attachment
        .initial
        .snapshot
        .as_ref()
        .map(|value| value.data.clone())
        .unwrap_or_default();
    for chunk in &attachment.initial.chunks {
        output.extend_from_slice(&chunk.data);
    }
    let deadline = tokio::time::Instant::now() + WIRE_RESPONSE_TIMEOUT;
    while !output.windows(needle.len()).any(|window| window == needle) {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(
            !remaining.is_zero(),
            "terminal output did not contain expected bytes: {output:?}"
        );
        let message = timeout(remaining, attachment.next_event())
            .await
            .unwrap()
            .unwrap();
        if message.event == event::TERMINAL_OUTPUT {
            let chunk: OutputChunk = yolo_runtime::protocol::from_value(message.payload).unwrap();
            output.extend_from_slice(&chunk.data);
        }
    }
    output
}

pub(crate) async fn next_event_of_type(
    attachment: &mut Attachment,
    event_type: &str,
) -> EventEnvelope {
    let deadline = tokio::time::Instant::now() + WIRE_RESPONSE_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "event {event_type} was not received");
        let message = timeout(remaining, attachment.next_event())
            .await
            .unwrap()
            .unwrap();
        if message.event == event_type {
            return message;
        }
    }
}
