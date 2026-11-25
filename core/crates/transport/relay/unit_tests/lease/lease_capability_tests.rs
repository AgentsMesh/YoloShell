use super::*;
use crate::lease::LeaseEventPayload;
use crate::lease::constants::{DEFAULT_TTL_MS, MAX_TTL_MS};
use crate::protocol::{EventEnvelope, event, method};
use crate::relay::control::ControlChannel;
use crate::relay::session_test_support::{host_status, running_session};
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

const HOLDER: &str = "capability-owner";
const INSTANCE: &str = "capability-instance";
const GENERATION: u64 = 23;

fn public_lease() -> WriterLease {
    WriterLease {
        holder_id: HOLDER.into(),
        controller_instance_id: Some(INSTANCE.into()),
        generation: GENERATION,
        expires_at_ms: i64::MAX,
        ttl_ms: MAX_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS,
    }
}

fn controller(
    host: &FakeHost,
    terminal_id: Uuid,
    writer_capability: WriterCapability,
) -> (LeaseController, mpsc::UnboundedReceiver<RelayStatus>) {
    let (status, events) = mpsc::unbounded_channel();
    (
        LeaseController {
            runtime: RelayRuntime::existing(host.client()),
            target: test_runtime_target(terminal_id),
            identity: LeaseIdentity {
                holder_id: HOLDER.into(),
                controller_instance_id: INSTANCE.into(),
            },
            writer_capability,
            lease: None,
            conflict_holder: None,
            retry_mode: LeaseAcquireMode::Observe,
            prepared_generation: None,
            status_announced: false,
            next_maintenance: Instant::now(),
            status: status.into(),
            ownership_uncertain: false,
            control_lost: false,
            lifecycle: SessionLifecycle::Active,
        },
        events,
    )
}

#[tokio::test]
async fn public_get_and_event_cannot_grant_control() {
    let terminal_id = Uuid::new_v4();
    let lease = public_lease();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(lease.clone())),
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
    ])
    .await;
    let (mut controller, mut statuses) =
        controller(&host, terminal_id, WriterCapability::generate());
    controller.recover_transport().await.unwrap();
    assert_eq!(controller.generation(), None);
    assert!(matches!(
        statuses.try_recv(),
        Ok(RelayStatus::ControlLost { .. })
    ));
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::GET_WRITER_LEASE,
            method::ACQUIRE_WRITER
        ]
    );

    let payload = LeaseEventPayload::new(lease, None);
    let acquired = EventEnvelope::new(event::WRITER_LEASE_ACQUIRED, None, None, &payload).unwrap();
    controller.observe_event(&acquired).unwrap();
    assert_eq!(controller.generation(), None);
}

#[tokio::test]
async fn pending_capability_recovers_after_acquire_ack_loss() {
    let terminal_id = Uuid::new_v4();
    let capability = WriterCapability::generate();
    let host = FakeHost::start(vec![
        Reply::LeaseThenDisconnect(GENERATION),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::StoredLease,
        Reply::Lease(GENERATION),
    ])
    .await;
    let (mut controller, _) = controller(&host, terminal_id, capability.clone());
    controller.acquire().await.unwrap();
    assert_eq!(controller.generation(), Some(GENERATION));
    let capabilities = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::ACQUIRE_WRITER)
        .map(|request| {
            request
                .decode_payload::<crate::worker::LeaseRequest>()
                .unwrap()
                .writer_capability
        })
        .collect::<Vec<_>>();
    assert_eq!(capabilities, [capability.clone(), capability]);
}

#[tokio::test]
async fn wrong_mutation_capability_fences_and_reconciles() {
    let terminal_id = Uuid::new_v4();
    let capability = WriterCapability::generate();
    let lease = public_lease();
    let host = FakeHost::start(vec![
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
        Reply::value(&Some(lease.clone())),
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
    ])
    .await;
    let (mut controller, mut statuses) = controller(&host, terminal_id, capability.clone());
    controller.set_writer_for_test(lease).unwrap();
    let mut control = ControlChannel::connect(controller.runtime.clone(), controller.target)
        .await
        .unwrap();
    let error = control
        .input(
            GENERATION,
            &WriterCapability::generate(),
            Uuid::new_v4(),
            b"denied",
        )
        .await
        .unwrap_err();
    let RelayError::Client(error) = error else {
        panic!("expected a remote ownership error");
    };
    assert!(controller.mutation_rejected(&error).await.unwrap());
    assert_eq!(controller.generation(), None);
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::ControlAcquired);
    assert!(matches!(
        statuses.try_recv(),
        Ok(RelayStatus::ControlLost { .. })
    ));
    assert_eq!(
        host.methods(),
        [
            method::TERMINAL_INPUT,
            method::GET_WRITER_LEASE,
            method::RENEW_WRITER
        ]
    );
}
