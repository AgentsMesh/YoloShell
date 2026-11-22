use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use super::runtime_intake_test_support::RoutedFixture;
use crate::agent::hooks::{EventSequence, HookEnvelope, HookViolation, HookProviderKind};
use crate::codex_hooks::{
    CodexDurableRejection, CodexEnqueueAdmission, CodexEnqueueDecision, CodexHookArtifacts,
    CodexHookIntake, CodexOutbox, CodexOutboxState,
};
use crate::worker::codex_hook::constants::CODEX_ACK_IDENTITY;
use crate::worker::codex_hook::error::CodexHookRejection;
use crate::worker::codex_hook::ingress_supervisor::CodexHookIngressSupervisor;
use uuid::Uuid;

const DURABLE_REJECTION_HOME: &str = "ingress-durable-rejection-home";
const IDEMPOTENT_RETRY_ATTEMPTS: usize = 2;
const SINGLE_DURABLE_REJECTION_MARKER: usize = 1;

#[path = "../../../test_support/unit/codex_hook/ingress/ingress_durable_rejection_assertions.rs"]
mod assertions;
use assertions::{assert_revoked, assert_snapshot_revision};

#[tokio::test]
async fn production_ingress_recovers_an_acknowledged_durable_rejection() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(DURABLE_REJECTION_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider = HookProviderKind::Claude;

    let rejected = endpoint
        .begin_exchange(&socket, &event)
        .finish_durable_rejection()
        .await;

    assert_eq!(rejected.ack.code, CODEX_ACK_IDENTITY);
    assert!(!rejected.ack.accepted);
    assert_eq!(rejected.ack.event_id, Some(rejected.event.event_id));
    assert_eq!(rejected.ack.sequence, Some(rejected.event.sequence.0));
    assert_eq!(rejected.event.identity, fixture.identity);
    assert_eq!(rejected.occurrence.provider, HookProviderKind::Claude);
    assert_revoked(&runtime, &rejected.event, HookViolation::ProviderMismatch);
    let expected_state = runtime.state().unwrap();
    let outbox = endpoint.outbox(&rejected.event);
    let before_restart = outbox.load().unwrap();
    assert_marker(
        &before_restart,
        &rejected.event,
        HookViolation::ProviderMismatch,
    );
    for _ in 0..IDEMPOTENT_RETRY_ATTEMPTS {
        assert!(matches!(
            runtime.ingest(rejected.event.clone(), fixture.process.pid),
            Err(CodexHookRejection::Violation(
                HookViolation::ProviderMismatch
            ))
        ));
        assert_eq!(runtime.state().unwrap(), expected_state);
        assert_eq!(outbox.load().unwrap(), before_restart);
    }
    supervisor.shutdown().await.unwrap();
    assert_snapshot_revision(
        &runtime.snapshot_path(),
        &rejected.event,
        HookViolation::ProviderMismatch,
    );

    let (recovered, report) = endpoint.restart_runtime(&fixture);
    assert_eq!(report, Default::default());
    assert_revoked(&recovered, &rejected.event, HookViolation::ProviderMismatch);
    assert_eq!(recovered.state().unwrap(), expected_state);
    assert_eq!(outbox.load().unwrap(), before_restart);

    endpoint.finish();
}

#[test]
fn cold_recovery_settles_a_rejection_left_after_outbox_commit() {
    let fixture = RoutedFixture::new();
    let pending = pending_provider_rejection(&fixture);

    let (recovered, replay) = fixture.restart_runtime();
    assert_eq!(replay, Default::default());
    assert_revoked(&recovered, &pending.event, HookViolation::ProviderMismatch);
    let router = fixture.router_for(&recovered);

    let intake_report = recovered.reconcile_routed_intake(&router).unwrap();
    assert_eq!(intake_report, Default::default());
    assert!(pending.intake.scan().unwrap().occurrences.is_empty());
    let settled = pending.outbox.load().unwrap();
    assert!(settled.pending_for_test().is_empty());
    assert!(settled.pending_occurrence_witness_count_for_test() == usize::default());
    assert!(settled.occurrence_receipt_count_for_test() == usize::default());
    assert_eq!(
        settled.durable_rejections_for_test().len(),
        SINGLE_DURABLE_REJECTION_MARKER
    );
    assert_eq!(settled.acknowledged_sequence(), pending.event.sequence.0);
    assert_marker(&settled, &pending.event, HookViolation::ProviderMismatch);
    drop(router);

    let expected_state = recovered.state().unwrap();
    let (restarted, replay) = fixture.restart_runtime();
    assert_eq!(replay, Default::default());
    assert_revoked(&restarted, &pending.event, HookViolation::ProviderMismatch);
    assert_eq!(restarted.state().unwrap(), expected_state);
    assert_eq!(pending.outbox.load().unwrap(), settled);
}

#[test]
fn recovery_rejects_missing_or_mismatched_rejection_markers() {
    for replacement in [None, Some(HookViolation::VersionMismatch)] {
        let fixture = RoutedFixture::new();
        let pending = pending_provider_rejection(&fixture);
        let mut state = pending.outbox.load().unwrap();
        state.durable_rejections_mut_for_test().clear();
        if let Some(violation) = replacement {
            state.durable_rejections_mut_for_test().push_back(
                CodexDurableRejection::capture_for_test(&pending.event, violation).unwrap(),
            );
        }
        crate::paths::atomic_write_json(pending.outbox.path(), &state).unwrap();

        assert!(fixture.try_restart_runtime().is_err());
    }
}

struct PendingRejection {
    intake: CodexHookIntake,
    outbox: CodexOutbox,
    event: HookEnvelope,
}

fn pending_provider_rejection(fixture: &RoutedFixture) -> PendingRejection {
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap();
    let intake =
        CodexHookIntake::for_identity(artifacts.intake_directory(), &fixture.identity).unwrap();
    let mut raw = fixture.start(FIRST_EVENT_SEQUENCE);
    raw.event_id = Uuid::nil();
    raw.sequence = EventSequence::ZERO;
    raw.provider = HookProviderKind::Claude;
    let occurrence = intake.persist_occurrence_for_test(raw).unwrap();
    let identity = intake
        .find_exact_occurrence(&occurrence)
        .unwrap()
        .unwrap()
        .durable_identity()
        .unwrap();
    let outbox = CodexOutbox::for_identity(artifacts.outbox_path(), &fixture.identity).unwrap();
    let event = outbox
        .enqueue_admitted_decision(
            occurrence,
            Some(&identity),
            |_, admission| {
                assert_eq!(admission, CodexEnqueueAdmission::New);
                Ok(CodexEnqueueDecision::rejected(
                    (),
                    HookViolation::ProviderMismatch,
                ))
            },
            |_, _, _| Ok(()),
        )
        .unwrap()
        .into_parts()
        .0;
    assert_marker(
        &outbox.load().unwrap(),
        &event,
        HookViolation::ProviderMismatch,
    );
    PendingRejection {
        intake,
        outbox,
        event,
    }
}

fn assert_marker(state: &CodexOutboxState, event: &HookEnvelope, violation: HookViolation) {
    let marker = state
        .durable_rejections_for_test()
        .front()
        .expect("durable rejection marker");
    assert_eq!(marker.event_id(), event.event_id);
    assert_eq!(marker.sequence(), event.sequence);
    assert_eq!(marker.violation(), violation);
}
