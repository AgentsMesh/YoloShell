use super::super::constants::{CODEX_ACK_IDENTITY, CODEX_ACK_REVOKED};
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{FIRST_SEQUENCE, HookViolation, HookProviderKind};
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use std::path::Path;

const BEFORE_COMMIT_HOME: &str = "ingress-post-enqueue-before-commit-home";
const AFTER_COMMIT_HOME: &str = "ingress-post-enqueue-after-commit-home";
const BEFORE_PUBLICATION_HOME: &str = "ingress-post-enqueue-before-publication-home";
const PREFLIGHT_RETRY_HOME: &str = "ingress-post-enqueue-preflight-retry-home";
const SINGLE_RETAINED_EVENT: usize = 1;

#[derive(Clone, Copy)]
enum PostEnqueueFailure {
    SnapshotBeforeCommit,
    SnapshotAfterCommit,
    BeforePublication,
}

#[tokio::test]
async fn snapshot_failure_before_commit_acks_the_outbox_assigned_rejection() {
    assert_assigned_rejection_ack(BEFORE_COMMIT_HOME, PostEnqueueFailure::SnapshotBeforeCommit)
        .await;
}

#[tokio::test]
async fn snapshot_failure_after_commit_acks_the_outbox_assigned_rejection() {
    assert_assigned_rejection_ack(AFTER_COMMIT_HOME, PostEnqueueFailure::SnapshotAfterCommit).await;
}

#[tokio::test]
async fn publication_failure_uses_the_durable_rejection_code_and_assigned_identity() {
    assert_assigned_rejection_ack(
        BEFORE_PUBLICATION_HOME,
        PostEnqueueFailure::BeforePublication,
    )
    .await;
}

#[tokio::test]
async fn later_preflight_failure_reuses_the_authoritative_rejection_ack() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(PREFLIGHT_RETRY_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let mut occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    occurrence.provider = HookProviderKind::Claude;
    let outbox = endpoint.outbox(&occurrence);
    arm_snapshot_failure(
        &outbox,
        &runtime.snapshot_path(),
        PostEnqueueFailure::SnapshotBeforeCommit,
    );

    let (first, durable_occurrence) = endpoint
        .begin_exchange(&socket, &occurrence)
        .finish_post_enqueue_failure_detailed()
        .await;
    let retry = endpoint
        .retry_existing(&socket, &durable_occurrence, PeerEvidence::WrongGeneration)
        .await;

    assert_eq!(retry, first);
    assert_eq!(retry.code, CODEX_ACK_IDENTITY);
    assert_retained_rejection(&outbox.load().unwrap());
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

async fn assert_assigned_rejection_ack(home: &str, failure: PostEnqueueFailure) {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(home);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let mut occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    occurrence.provider = HookProviderKind::Claude;
    let outbox = endpoint.outbox(&occurrence);
    let snapshot_path = runtime.snapshot_path();
    let snapshot_before = std::fs::read(&snapshot_path).unwrap();
    arm_snapshot_failure(&outbox, &snapshot_path, failure);

    let ack = endpoint
        .begin_exchange(&socket, &occurrence)
        .finish_post_enqueue_failure()
        .await;

    let state = outbox.load().unwrap();
    let assigned = assert_retained_rejection(&state);
    assert!(!ack.accepted);
    assert_eq!(ack.event_id, Some(assigned.event_id));
    assert_eq!(ack.sequence, Some(assigned.sequence.0));
    assert_eq!(ack.code, CODEX_ACK_IDENTITY);
    assert_ne!(ack.code, CODEX_ACK_REVOKED);
    assert!(runtime.state().is_err());
    assert_snapshot_stage(&snapshot_path, &snapshot_before, failure);

    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

fn arm_snapshot_failure(outbox: &CodexOutbox, snapshot: &Path, failure: PostEnqueueFailure) {
    let snapshot = snapshot.to_path_buf();
    crate::codex_hooks::set_outbox_commit_hook(
        outbox.path().to_path_buf(),
        move || match failure {
            PostEnqueueFailure::SnapshotBeforeCommit => {
                crate::codex_hooks::fail_snapshot_before_commit(&snapshot)
            }
            PostEnqueueFailure::SnapshotAfterCommit => {
                crate::codex_hooks::fail_snapshot_after_commit(&snapshot)
            }
            PostEnqueueFailure::BeforePublication => {
                crate::codex_hooks::remove_snapshot_lock_before_publication(&snapshot)
            }
        },
    );
}

fn assert_retained_rejection(state: &CodexOutboxState) -> &crate::agent::hooks::HookEnvelope {
    assert_eq!(state.pending_for_test().len(), SINGLE_RETAINED_EVENT);
    assert_eq!(
        state.durable_rejections_for_test().len(),
        SINGLE_RETAINED_EVENT
    );
    let assigned = state.pending_for_test().front().unwrap();
    let marker = state.durable_rejections_for_test().front().unwrap();
    assert!(!assigned.event_id.is_nil());
    assert!(assigned.sequence.0 >= FIRST_SEQUENCE);
    assert_eq!(marker.event_id(), assigned.event_id);
    assert_eq!(marker.sequence(), assigned.sequence);
    assert_eq!(marker.violation(), HookViolation::ProviderMismatch);
    assigned
}

fn assert_snapshot_stage(path: &Path, before: &[u8], failure: PostEnqueueFailure) {
    let after = std::fs::read(path).unwrap();
    match failure {
        PostEnqueueFailure::SnapshotBeforeCommit => assert_eq!(after, before),
        PostEnqueueFailure::SnapshotAfterCommit | PostEnqueueFailure::BeforePublication => {
            assert_ne!(after, before)
        }
    }
}
