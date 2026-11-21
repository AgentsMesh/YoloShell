use super::super::constants::{
    CODEX_ACK_PROCESS, CODEX_ACK_PROTOCOL, CODEX_INTAKE_ENUMERATION_ERROR,
};
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::{IngressEndpoint, committed_runtime};
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::EventSequence;
use std::path::{Path, PathBuf};

const ADMISSION_HOME: &str = "ingress-commit-fence-admission-home";
const EXECUTABLE_HOME: &str = "ingress-commit-fence-executable-home";
const OCCURRENCE_HOME: &str = "ingress-commit-fence-occurrence-home";
const REVOKED_HOME: &str = "ingress-commit-fence-revoked-home";
const REPLACEMENT_FILE: &str = "commit-fence-replacement";
const SINGLE_OCCURRENCE: usize = 1;

#[derive(Clone, Copy)]
enum Replacement {
    Admission,
    Executable,
    Occurrence,
}

#[derive(Clone, Copy)]
enum Delivery {
    Rejected,
    Revoked,
}

#[tokio::test]
async fn same_payload_admission_replacement_revokes_the_commit() {
    assert_commit_rejected(
        ADMISSION_HOME,
        Replacement::Admission,
        CODEX_ACK_PROTOCOL,
        Delivery::Rejected,
    )
    .await;
}

#[tokio::test]
async fn same_content_executable_replacement_revokes_the_commit() {
    assert_commit_rejected(
        EXECUTABLE_HOME,
        Replacement::Executable,
        CODEX_ACK_PROCESS,
        Delivery::Rejected,
    )
    .await;
}

#[tokio::test]
async fn same_payload_raw_occurrence_replacement_revokes_the_commit() {
    assert_commit_rejected(
        OCCURRENCE_HOME,
        Replacement::Occurrence,
        CODEX_ACK_PROTOCOL,
        Delivery::Rejected,
    )
    .await;
}

#[tokio::test]
async fn admission_replacement_and_revocation_before_ack_leave_no_committed_event() {
    assert_commit_rejected(
        REVOKED_HOME,
        Replacement::Admission,
        CODEX_ACK_PROTOCOL,
        Delivery::Revoked,
    )
    .await;
}

async fn assert_commit_rejected(
    home: &str,
    replacement: Replacement,
    expected_code: &str,
    delivery: Delivery,
) {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(home);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let router = endpoint.start_router(&supervisor, &fixture).await.unwrap();
    let runtime = committed_runtime(std::sync::Arc::clone(&router), &fixture);
    let mut intake_pause = Some(router.pause_next_intake_round_for_test().await);
    let before = runtime.state().unwrap().unwrap();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let outbox = endpoint.outbox(&event);
    let outbox_path = outbox.path().to_path_buf();
    let mutation = replacement_path(&endpoint, &fixture, replacement);
    crate::codex_hooks::set_outbox_commit_hook(outbox_path, move || match replacement {
        Replacement::Admission | Replacement::Executable => replace_same_content(&mutation),
        Replacement::Occurrence => replace_only_entry(&mutation),
    });

    let ack_gate =
        matches!(delivery, Delivery::Revoked).then(|| runtime.gate_next_ack_write_for_test());
    let exchange = endpoint.begin_exchange(&socket, &event);
    let shutdown = if let Some(gate) = ack_gate {
        gate.wait_until_entered().await;
        let mut shutdown = Box::pin(supervisor.shutdown());
        assert!(futures::poll!(&mut shutdown).is_pending());
        gate.release();
        exchange.finish_without_ack().await;
        drop(intake_pause.take());
        Some(shutdown.await)
    } else {
        let ack = exchange.finish_rejected().await;
        assert_eq!(ack.code, expected_code);
        None
    };
    assert_eq!(runtime.state().unwrap().unwrap(), before);
    assert!(outbox.load().unwrap().pending_for_test().is_empty());
    let (restarted, replay) = endpoint.restart_runtime(&fixture);
    assert_eq!(replay, Default::default());
    assert_eq!(
        restarted.state().unwrap().unwrap().last_sequence,
        EventSequence::FIRST
    );
    let intake = restarted.scan_unrouted_intake().unwrap();
    assert_eq!(intake.deferred, SINGLE_OCCURRENCE);
    assert_eq!(
        intake.applied + intake.replayed + intake.stale,
        usize::default()
    );
    drop(intake_pause.take());
    let shutdown = match shutdown {
        Some(result) => result,
        None => supervisor.shutdown().await,
    };
    match replacement {
        Replacement::Admission => {
            assert!(
                format!("{:#}", shutdown.unwrap_err()).contains(CODEX_INTAKE_ENUMERATION_ERROR)
            );
        }
        Replacement::Executable | Replacement::Occurrence => shutdown.unwrap(),
    }
    endpoint.finish();
}

fn replacement_path(
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
    replacement: Replacement,
) -> PathBuf {
    match replacement {
        Replacement::Admission => endpoint.admission_path(fixture),
        Replacement::Executable => endpoint.admission_context(fixture).hook_executable,
        Replacement::Occurrence => crate::codex_hooks::CodexHookArtifacts::new(
            &endpoint.paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            fixture.identity.invocation_id,
        )
        .unwrap()
        .intake_directory(),
    }
}

fn replace_only_entry(directory: &Path) {
    let entries = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), SINGLE_OCCURRENCE);
    replace_same_content(entries.first().unwrap());
}

fn replace_same_content(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    let permissions = std::fs::metadata(path).unwrap().permissions();
    let replacement = path.parent().unwrap().join(REPLACEMENT_FILE);
    std::fs::write(&replacement, bytes).unwrap();
    std::fs::set_permissions(&replacement, permissions.clone()).unwrap();
    std::fs::rename(replacement, path).unwrap();
    assert_eq!(std::fs::metadata(path).unwrap().permissions(), permissions);
}
