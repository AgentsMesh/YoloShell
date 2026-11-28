use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{HookEnvelope, HookProviderKind};
use crate::codex_hooks::{CodexHookIntake, CodexOutbox, CodexOutboxState};
use std::sync::Arc;

const MISSING_HOME: &str = "ingress-post-rename-missing-home";
const OLD_HOME: &str = "ingress-post-rename-old-home";
const PERSISTENT_HOME: &str = "ingress-post-rename-persistent-home";
const SINGLE_EVENT: usize = 1;

type Server = super::super::server::CodexHookServer;
type Runtime = super::super::runtime::CodexHookRuntime;

#[derive(Clone, Copy)]
enum CrashModel {
    Missing,
    Old,
    PersistentBarrierFailure,
}

#[tokio::test]
async fn missing_create_namespace_closes_consecutive_connections_without_ack() {
    assert_indeterminate_connections(MISSING_HOME, CrashModel::Missing).await;
}

#[tokio::test]
async fn old_exchange_namespace_closes_consecutive_connections_without_ack() {
    assert_indeterminate_connections(OLD_HOME, CrashModel::Old).await;
}

#[tokio::test]
async fn persistent_reproof_barrier_failure_closes_consecutive_connections_without_ack() {
    assert_indeterminate_connections(PERSISTENT_HOME, CrashModel::PersistentBarrierFailure).await;
}

async fn assert_indeterminate_connections(home: &str, crash: CrashModel) {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(home);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider = HookProviderKind::Claude;
    let outbox = endpoint.outbox(&event);
    if matches!(crash, CrashModel::Old) {
        crate::paths::atomic_write_json(outbox.path(), &CodexOutboxState::default()).unwrap();
    }
    let before = outbox.load().unwrap();
    let snapshot_before = std::fs::read(runtime.snapshot_path()).unwrap();
    arm_failure(outbox.path(), crash);

    let pending = endpoint.begin_exchange(server.socket_path(), &event);
    let stream = server.accept().await.unwrap().0;
    let handled = tokio::spawn({
        let server = Arc::clone(&server);
        async move { server.handle_stream(stream).await }
    });
    let occurrence = pending.finish_indeterminate().await;
    assert!(handled.await.unwrap().unwrap().into_parts().1.is_err());
    server.settlements().wait_idle().await;

    assert!(runtime.storage_failed().unwrap());
    assert_eq!(runtime.state().unwrap(), None);
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
    assert_eq!(
        std::fs::read(runtime.snapshot_path()).unwrap(),
        snapshot_before
    );
    assert_raw(&endpoint, &occurrence);
    let after_first = assert_first_namespace(&outbox, &before, crash);

    endpoint
        .retry_without_ack(server.as_ref(), &occurrence)
        .await;

    match after_first {
        Some(expected) => assert_eq!(outbox.load().unwrap(), expected),
        None => assert!(!outbox.path().exists()),
    }
    assert_raw(&endpoint, &occurrence);
    assert!(runtime.storage_failed().unwrap());
    drop(runtime);
    drop(server);
    endpoint.finish();
}

fn arm_failure(path: &std::path::Path, crash: CrashModel) {
    crate::codex_hooks::set_outbox_commit_hook(path.to_path_buf(), || {
        crate::paths::test_support::fail_next_directory_sync();
    });
    let path = path.to_path_buf();
    crate::codex_hooks::set_outbox_reproof_hook(path.clone(), move || match crash {
        CrashModel::Missing => std::fs::remove_file(path).unwrap(),
        CrashModel::Old => {
            crate::paths::atomic_write_json(&path, &CodexOutboxState::default()).unwrap();
        }
        CrashModel::PersistentBarrierFailure => {
            crate::paths::test_support::fail_next_directory_sync()
        }
    });
}

fn assert_first_namespace(
    outbox: &CodexOutbox,
    before: &CodexOutboxState,
    crash: CrashModel,
) -> Option<CodexOutboxState> {
    match crash {
        CrashModel::Missing => {
            assert!(!outbox.path().exists());
            None
        }
        CrashModel::Old => {
            let after = outbox.load().unwrap();
            assert_eq!(&after, before);
            Some(after)
        }
        CrashModel::PersistentBarrierFailure => {
            let after = outbox.load().unwrap();
            assert_eq!(after.pending_for_test().len(), SINGLE_EVENT);
            assert_eq!(after.durable_rejections_for_test().len(), SINGLE_EVENT);
            Some(after)
        }
    }
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

fn assert_raw(endpoint: &IngressEndpoint, occurrence: &HookEnvelope) {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        &endpoint.paths,
        occurrence.identity.terminal_id,
        occurrence.identity.runtime_epoch,
        occurrence.identity.invocation_id,
    )
    .unwrap();
    let intake =
        CodexHookIntake::for_identity(artifacts.intake_directory(), &occurrence.identity).unwrap();
    assert!(intake.find_exact_occurrence(occurrence).unwrap().is_some());
}
