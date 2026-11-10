use super::super::constants::CODEX_ACK_IDENTITY;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{HookEnvelope, HookViolation, HookProviderKind};
use crate::codex_hooks::CodexHookIntake;
use std::sync::Arc;

const POST_RENAME_SYNC_HOME: &str = "ingress-post-rename-sync-home";
const SINGLE_RETAINED_EVENT: usize = 1;
const REGISTRATION_AND_REJECTION_EVENTS: usize = 2;

type Server = super::super::server::CodexHookServer;
type Runtime = super::super::runtime::CodexHookRuntime;

#[tokio::test]
async fn post_rename_sync_failure_converges_with_the_assigned_rejection() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(POST_RENAME_SYNC_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider = HookProviderKind::Claude;
    let outbox = endpoint.outbox(&event);
    assert_eq!(
        outbox.load().unwrap().acknowledged_sequence(),
        crate::agent::hooks::FIRST_SEQUENCE
    );
    crate::codex_hooks::set_outbox_commit_hook(outbox.path().to_path_buf(), || {
        crate::paths::test_support::fail_next_directory_sync();
    });

    let pending = endpoint.begin_exchange(server.socket_path(), &event);
    let stream = server.accept().await.unwrap().0;
    let handled = tokio::spawn({
        let server = Arc::clone(&server);
        async move { server.handle_stream(stream).await }
    });
    let (first, occurrence) = pending.finish_post_enqueue_failure_detailed().await;
    assert!(handled.await.unwrap().unwrap().into_parts().1.is_ok());
    server.settlements().wait_idle().await;
    let assigned = assert_settled_rejection(&outbox.load().unwrap());
    assert_rejection_ack(&first, &assigned);
    assert_raw(&endpoint, &occurrence, false);
    assert_revoked(&runtime, &assigned);
    assert!(!runtime.storage_failed().unwrap());
    drop(runtime);
    drop(server);

    let (recovered_server, recovered) = bound_runtime(&endpoint, &fixture).await;
    assert_revoked(&recovered, &assigned);
    assert_eq!(assert_settled_rejection(&outbox.load().unwrap()), assigned);
    assert_raw(&endpoint, &occurrence, false);
    assert!(!recovered.storage_failed().unwrap());
    drop(recovered);
    drop(recovered_server);
    endpoint.finish();
}

async fn bound_runtime(
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
) -> (Arc<Server>, Arc<Runtime>) {
    let supervisor = CodexHookIngressSupervisor::new();
    let server = Arc::new(endpoint.bind_server(&supervisor, fixture).await);
    let runtime = endpoint.bound_runtime(server.as_ref(), fixture).await;
    (server, runtime)
}

fn assert_settled_rejection(state: &crate::codex_hooks::CodexOutboxState) -> HookEnvelope {
    assert!(state.pending_for_test().is_empty());
    assert!(state.occurrence_receipt_count_for_test() == usize::default());
    assert_eq!(
        state.acknowledged_events_for_test().len(),
        REGISTRATION_AND_REJECTION_EVENTS
    );
    assert_eq!(
        state.durable_rejections_for_test().len(),
        SINGLE_RETAINED_EVENT
    );
    let assigned = state.acknowledged_events_for_test().back().unwrap().clone();
    let marker = state.durable_rejections_for_test().back().unwrap();
    assert_eq!(marker.event_id(), assigned.event_id);
    assert_eq!(marker.sequence(), assigned.sequence);
    assert_eq!(marker.violation(), HookViolation::ProviderMismatch);
    assigned
}

fn assert_rejection_ack(ack: &super::super::protocol::CodexHookAck, assigned: &HookEnvelope) {
    assert!(!ack.accepted);
    assert_eq!(ack.event_id, Some(assigned.event_id));
    assert_eq!(ack.sequence, Some(assigned.sequence.0));
    assert_eq!(ack.code, CODEX_ACK_IDENTITY);
}

fn assert_revoked(runtime: &Runtime, assigned: &HookEnvelope) {
    let state = runtime.state().unwrap().unwrap();
    assert!(state.revoked);
    let (rejected, proof) = state
        .revocation
        .as_ref()
        .and_then(|evidence| evidence.hook_rejection())
        .expect("hook revocation evidence");
    assert_eq!(rejected, assigned);
    assert_eq!(proof.event_id(), assigned.event_id);
    assert_eq!(proof.sequence(), assigned.sequence);
    assert_eq!(proof.violation(), HookViolation::ProviderMismatch);
}

fn assert_raw(endpoint: &IngressEndpoint, occurrence: &HookEnvelope, retained: bool) {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        &endpoint.paths,
        occurrence.identity.terminal_id,
        occurrence.identity.runtime_epoch,
        occurrence.identity.invocation_id,
    )
    .unwrap();
    let intake =
        CodexHookIntake::for_identity(artifacts.intake_directory(), &occurrence.identity).unwrap();
    assert_eq!(
        intake.find_exact_occurrence(occurrence).unwrap().is_some(),
        retained
    );
}
