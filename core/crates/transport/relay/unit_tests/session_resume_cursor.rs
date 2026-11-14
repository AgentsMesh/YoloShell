use super::*;
use crate::model::TerminalRuntimeStatus;
use crate::protocol::method;
use crate::relay::session_test_support::{
    GENERATION, closed_resize_channel, host_status, running_session, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, Reply};
use crate::worker::{AttachRequest, FORCE_SNAPSHOT_AFTER_SEQUENCE};

const OLD_RUNTIME_CURSOR: u64 = 41;
const RESUMED_SNAPSHOT: &[u8] = b"resumed runtime snapshot";
const RESUMED_HOLDER: &str = "resumed-holder";

#[tokio::test]
async fn resumed_runtime_discards_the_previous_epoch_cursor() {
    let terminal_id = Uuid::new_v4();
    let mut unreachable = running_session(terminal_id);
    unreachable.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Unreachable;
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&unreachable),
        Reply::value(&running_session(terminal_id)),
        Reply::attachment(snapshot_attach(RESUMED_SNAPSHOT), Vec::new()),
        Reply::Lease(GENERATION),
        Reply::value(&()),
    ])
    .await;

    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        RelayOptions::new(terminal_id, RESUMED_HOLDER)
            .unwrap()
            .after_sequence(OLD_RUNTIME_CURSOR),
        tokio::io::empty(),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);

    let attach = host
        .requests()
        .into_iter()
        .find(|request| request.method == method::ATTACH_SESSION)
        .unwrap()
        .decode_payload::<AttachRequest>()
        .unwrap();
    assert_eq!(attach.after_seq, FORCE_SNAPSHOT_AFTER_SEQUENCE);
}
