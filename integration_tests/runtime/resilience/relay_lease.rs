use super::*;
use tokio::io::DuplexStream;
use tokio::sync::mpsc;
use yolo_relay::{RelayExit, RelayOptions, RelayStatus, relay_session_with_status};
use yolo_runtime::WriterCapability;

const IO_CAPACITY: usize = 64 * 1024;
const LEASE_TIMEOUT: Duration = Duration::from_secs(5);
const SETTLE_DELAY: Duration = Duration::from_millis(50);
const RENEWAL_SETTLE_MARGIN: Duration = Duration::from_millis(250);
const HOLDER_ONE: &str = "stable-holder-one";

struct RunningRelay {
    input: DuplexStream,
    _output: DuplexStream,
    statuses: mpsc::UnboundedReceiver<RelayStatus>,
    task: tokio::task::JoinHandle<Result<RelayExit, yolo_relay::RelayError>>,
}

fn start_relay(harness: &Harness, options: RelayOptions) -> RunningRelay {
    let (input, input_reader) = tokio::io::duplex(IO_CAPACITY);
    let (output, output_reader) = tokio::io::duplex(IO_CAPACITY);
    let (_resize_sender, resize_events) = mpsc::channel(1);
    let (status_sender, statuses) = mpsc::unbounded_channel();
    let client = harness.client();
    let task = tokio::spawn(relay_session_with_status(
        client,
        options,
        input_reader,
        output,
        resize_events,
        status_sender,
    ));
    RunningRelay {
        input,
        _output: output_reader,
        statuses,
        task,
    }
}

async fn next_status(relay: &mut RunningRelay) -> RelayStatus {
    timeout(LEASE_TIMEOUT, relay.statuses.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn stop_relay(relay: RunningRelay) -> RelayExit {
    drop(relay.input);
    timeout(LEASE_TIMEOUT, relay.task)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_holder_rebuild_waits_for_the_live_predecessor_then_acquires() {
    let harness = Harness::start().await;
    let session = harness.create_cat("same-holder-fencing").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let mut first = start_relay(&harness, RelayOptions::new(id, HOLDER_ONE).unwrap());
    assert_eq!(next_status(&mut first).await, RelayStatus::ControlAcquired);
    let original = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(SETTLE_DELAY).await;

    let mut replacement = start_relay(&harness, RelayOptions::new(id, HOLDER_ONE).unwrap());
    assert_eq!(
        next_status(&mut replacement).await,
        RelayStatus::ReadOnly {
            holder_id: Some(HOLDER_ONE.into())
        }
    );
    let current = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.generation, original.generation);
    assert_eq!(stop_relay(first).await, RelayExit::InputClosed);
    assert_eq!(
        next_status(&mut replacement).await,
        RelayStatus::ControlAcquired
    );
    let successor = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    assert!(successor.generation > original.generation);
    assert_ne!(
        successor.controller_instance_id,
        original.controller_instance_id
    );
    assert_eq!(stop_relay(replacement).await, RelayExit::InputClosed);
    assert!(
        harness
            .client()
            .get_writer_lease(target)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_takeover_uses_observed_generation_and_releases_on_exit() {
    let harness = Harness::start().await;
    let session = harness.create_cat("explicit-takeover").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let original = harness
        .client()
        .acquire_writer_for_instance(
            target,
            HOLDER_ONE,
            Some("old-instance".into()),
            WriterCapability::generate(),
            None,
        )
        .await
        .unwrap();
    let options = RelayOptions::new(id, HOLDER_ONE).unwrap().take_control(true);
    let mut relay = start_relay(&harness, options);
    assert_eq!(next_status(&mut relay).await, RelayStatus::ControlAcquired);
    let current = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.holder_id, HOLDER_ONE);
    assert!(current.generation > original.generation);
    let stale_cleanup = harness
        .client()
        .release_writer(&original, target)
        .await
        .unwrap_err();
    assert!(matches!(
        stale_cleanup,
        yolo_client::ClientError::Remote { code, .. }
            if code == crate::lease_error_code::CONTROLLER_INSTANCE_MISMATCH
    ));
    assert_eq!(
        harness.client().get_writer_lease(target).await.unwrap(),
        Some(current)
    );
    assert_eq!(stop_relay(relay).await, RelayExit::InputClosed);
    assert!(
        harness
            .client()
            .get_writer_lease(target)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writer_lease_is_renewed_while_the_relay_is_attached() {
    let harness = Harness::start().await;
    let session = harness.create_cat("lease-renewal").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let mut relay = start_relay(&harness, RelayOptions::new(id, HOLDER_ONE).unwrap());
    assert_eq!(next_status(&mut relay).await, RelayStatus::ControlAcquired);
    let before = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    let renewal_delay = Duration::from_millis(before.renew_after_ms) + RENEWAL_SETTLE_MARGIN;
    tokio::time::sleep(renewal_delay).await;
    let after = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.generation, before.generation);
    assert!(after.expires_at_ms > before.expires_at_ms);
    assert_eq!(stop_relay(relay).await, RelayExit::InputClosed);
}
