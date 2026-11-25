use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn completed_attachment_active_custody_is_host_authenticated() {
    let ActiveCompletedFixture {
        _directory,
        host,
        record,
        manifest,
        runtime,
        listener: _listener,
        ..
    } = active_completed_fixture();
    assert_eq!(
        host.live_runtime_record(record.terminal_id).await.unwrap(),
        record
    );
    assert!(runtime_view_matches(&runtime, &record));
    assert!(host.rpc_custody_ready(&record));
    let observation = RuntimeObservation::authenticated(runtime, &record);
    let completed = host
        .completed_attachment_view(&manifest, Some(&observation))
        .await;

    assert_eq!(
        completed,
        Some(crate::model::CompletedAttachmentView {
            runtime_epoch: record.runtime_epoch,
        })
    );
}

#[tokio::test]
async fn completed_attachment_released_natural_exit_is_host_authenticated() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let exit = read_json(&host.paths.exit_record(record.terminal_id)).unwrap();
    let manifest = completed_test_manifest(&host, record.terminal_id);
    let runtime = completed_test_runtime(&record, exit, None);

    let observation = RuntimeObservation::unauthenticated(runtime);
    let completed = host
        .completed_attachment_view(&manifest, Some(&observation))
        .await;

    assert_eq!(
        completed,
        Some(crate::model::CompletedAttachmentView {
            runtime_epoch: record.runtime_epoch,
        })
    );
}

#[tokio::test]
async fn completed_attachment_rejects_malformed_or_archived_state() {
    const OUTPUT_SEQUENCE_MISMATCH_INCREMENT: u64 = 1;

    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let exit: ExitRecord = read_json(&host.paths.exit_record(record.terminal_id)).unwrap();
    let manifest = completed_test_manifest(&host, record.terminal_id);
    let runtime = completed_test_runtime(&record, exit, None);

    let mut archived = manifest.clone();
    archived.archived = true;
    let observation = RuntimeObservation::unauthenticated(runtime.clone());
    assert!(
        host.completed_attachment_view(&archived, Some(&observation))
            .await
            .is_none()
    );

    let mut wrong_epoch = runtime.clone();
    wrong_epoch.exit.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
    let mut wrong_reason = runtime.clone();
    wrong_reason.exit.as_mut().unwrap().reason = ExitReason::WorkerCrash;
    let mut wrong_sequence = runtime.clone();
    wrong_sequence.last_output_sequence = wrong_sequence
        .last_output_sequence
        .checked_add(OUTPUT_SEQUENCE_MISMATCH_INCREMENT)
        .unwrap();
    for rejected in [wrong_epoch, wrong_reason, wrong_sequence] {
        let observation = RuntimeObservation::unauthenticated(rejected);
        assert!(
            host.completed_attachment_view(&manifest, Some(&observation))
                .await
                .is_none()
        );
    }
}

#[tokio::test]
async fn get_session_publishes_active_completed_attachment() {
    const PUBLIC_STATUS_REQUESTS: usize = 1;
    let ActiveCompletedFixture {
        _directory,
        host,
        record,
        manifest,
        runtime,
        listener,
    } = active_completed_fixture();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    let server = tokio::spawn(serve_completed_worker_status_count(
        listener,
        record.clone(),
        runtime,
        PUBLIC_STATUS_REQUESTS,
    ));

    let response = host
        .dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: record.terminal_id,
            },
        ))
        .await;

    assert!(response.ok, "{:?}", response.error);
    let view: TerminalView = response.decode_payload().unwrap();
    assert_eq!(
        view.completed_attachment,
        Some(crate::model::CompletedAttachmentView {
            runtime_epoch: record.runtime_epoch,
        })
    );
    await_completed_worker(server).await;
}

#[tokio::test]
async fn get_session_publishes_released_completed_attachment_fallback() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let manifest = completed_test_manifest(&host, record.terminal_id);
    lock_recover(&host.catalog).insert(&manifest).unwrap();

    let response = host
        .dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: record.terminal_id,
            },
        ))
        .await;

    assert!(response.ok, "{:?}", response.error);
    let view: TerminalView = response.decode_payload().unwrap();
    assert_eq!(view.runtime.as_ref().unwrap().status, TerminalRuntimeStatus::Exited);
    assert_eq!(
        view.completed_attachment,
        Some(crate::model::CompletedAttachmentView {
            runtime_epoch: record.runtime_epoch,
        })
    );
}
