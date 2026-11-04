use super::support::*;
use crate::agent::hooks::{ProcessIdentity, ProcessState};
use crate::local_unix::LocalUnixStream;
use std::sync::Arc;

#[tokio::test]
async fn terminal_commit_waits_for_an_older_accepted_response() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let response_gate = runtime.gate_next_response_for_test();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    write_bytes_frame(&mut client, &yolo_capnp_codec::encode(&event).unwrap()).await;
    response_gate.wait_until_entered().await;

    let exit_runtime = Arc::clone(&runtime);
    let observed = ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);
    let mut terminal =
        tokio::task::spawn_blocking(move || exit_runtime.observe_process_exit(observed));
    assert_terminal_blocked(&mut terminal).await;

    response_gate.release();
    let ack = read_ack(&mut client).await;
    let outcome = handler.await.unwrap().unwrap();
    let (publication, response) = outcome.into_parts();
    response.unwrap();
    let terminal = terminal.await.unwrap().unwrap();

    assert!(ack.accepted);
    assert!(publication.is_some());
    assert_eq!(
        terminal.disposition,
        crate::agent::hooks::ProcessExitDisposition::Unconfirmed
    );
    assert_eq!(
        runtime.state().unwrap().unwrap().process,
        ProcessState::Exited
    );
}

#[tokio::test]
async fn cancelled_accepted_response_fails_closed_before_terminal() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let mut authority = runtime.subscribe_authority();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    saturate_ack_channel(&server);
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    write_bytes_frame(&mut client, &yolo_capnp_codec::encode(&event).unwrap()).await;
    authority.changed().await.unwrap();

    let mut terminal = spawn_process_exit(&fixture, Arc::clone(&runtime));
    assert_terminal_blocked(&mut terminal).await;
    handler.abort();
    assert!(matches!(handler.await, Err(error) if error.is_cancelled()));

    wait_for_storage_unavailable(&mut authority).await;
    assert!(matches!(
        terminal.await.unwrap(),
        Err(super::super::error::CodexHookRejection::Storage(_))
    ));
}

#[tokio::test]
async fn accepted_response_timeout_releases_waiting_terminal() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let mut authority = runtime.subscribe_authority();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    saturate_ack_channel(&server);
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    write_bytes_frame(&mut client, &yolo_capnp_codec::encode(&event).unwrap()).await;
    authority.changed().await.unwrap();

    let mut terminal = spawn_process_exit(&fixture, Arc::clone(&runtime));
    assert_terminal_blocked(&mut terminal).await;
    let outcome = handler.await.unwrap().unwrap();
    let (disposition, response) = outcome.into_parts();
    let terminal = terminal.await.unwrap().unwrap();

    assert_eq!(
        disposition,
        Some(crate::agent::hooks::ApplyDisposition::Applied)
    );
    assert!(response.is_err());
    assert_eq!(
        terminal.disposition,
        crate::agent::hooks::ProcessExitDisposition::Unconfirmed
    );
    assert_eq!(
        runtime.state().unwrap().unwrap().process,
        ProcessState::Exited
    );
    assert!(!runtime.storage_failed().unwrap());
}

#[tokio::test]
async fn fresh_registry_recovers_durable_event_before_older_ack_settlement() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let mut authority = runtime.subscribe_authority();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    saturate_ack_channel(&server);
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    write_bytes_frame(&mut client, &yolo_capnp_codec::encode(&event).unwrap()).await;
    authority.changed().await.unwrap();

    let restart = spawn_runtime_restart(&fixture);
    let (restarted, report) =
        tokio::time::timeout(super::super::constants::CODEX_REQUEST_TIMEOUT, restart)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    assert_eq!(report, Default::default());
    assert_eq!(
        restarted.state().unwrap().unwrap().last_sequence,
        event.sequence
    );
    handler.abort();
    assert!(matches!(handler.await, Err(error) if error.is_cancelled()));

    wait_for_storage_unavailable(&mut authority).await;
}

fn spawn_process_exit(
    fixture: &Fixture,
    runtime: Arc<super::super::runtime::CodexHookRuntime>,
) -> tokio::task::JoinHandle<
    Result<
        super::super::runtime::CodexProcessExitPublication,
        super::super::error::CodexHookRejection,
    >,
> {
    let observed = ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);
    tokio::task::spawn_blocking(move || runtime.observe_process_exit(observed))
}

fn spawn_runtime_restart(
    fixture: &Fixture,
) -> tokio::task::JoinHandle<
    anyhow::Result<(
        Arc<super::super::runtime::CodexHookRuntime>,
        super::super::runtime::ReplayReport,
    )>,
> {
    let paths = fixture.paths.clone();
    let shell = fixture.shell;
    let identity = fixture.identity.clone();
    let process_source = fixture.process_source();
    tokio::task::spawn_blocking(move || {
        super::super::runtime::CodexHookRuntime::new_test(
            &paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            shell,
            identity,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            process_source,
        )
    })
}

async fn assert_terminal_blocked<T>(terminal: &mut tokio::task::JoinHandle<T>) {
    assert!(
        tokio::time::timeout(
            super::super::constants::CODEX_SOCKET_PROBE_TIMEOUT,
            terminal,
        )
        .await
        .is_err()
    );
}

async fn wait_for_storage_unavailable(
    authority: &mut tokio::sync::watch::Receiver<super::super::publication::CodexAuthorityView>,
) {
    tokio::time::timeout(super::super::constants::CODEX_REQUEST_TIMEOUT, async {
        loop {
            if matches!(
                authority.borrow().clone(),
                super::super::publication::CodexAuthorityView::StorageUnavailable
            ) {
                return;
            }
            authority.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}
