use super::*;
use crate::protocol::{event, method};
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, empty_attach, empty_attach_for, options,
    owned_with_attachment, session_event, test_runtime_epoch,
};
use crate::relay::test_support::{FakeHost, Reply};

const EXPECTED_ATTACHMENT_REQUESTS: usize = 2;

#[tokio::test]
async fn attachment_epoch_relay_rejects_replaced_runtime_before_initial_output() {
    let terminal_id = Uuid::new_v4();
    let replacement_epoch = Uuid::new_v4();
    let mut replies = owned_with_attachment(
        terminal_id,
        Reply::attachment(
            empty_attach(),
            vec![session_event(event::OUTPUT_GAP, terminal_id)],
        ),
    );
    replies.extend([
        Reply::attachment(empty_attach_for(replacement_epoch), Vec::new()),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let error = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        RelayError::RuntimeEpochChanged { expected, actual }
            if expected == test_runtime_epoch() && actual == replacement_epoch
    ));
    assert_eq!(
        host.methods()
            .iter()
            .filter(|candidate| candidate.as_str() == method::ATTACH_SESSION)
            .count(),
        EXPECTED_ATTACHMENT_REQUESTS
    );
}
