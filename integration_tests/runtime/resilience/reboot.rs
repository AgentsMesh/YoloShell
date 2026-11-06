use super::*;

const PREVIOUS_BOOT_ID: &str = "previous-boot";
const NATURAL_EXIT_SCREEN: &[u8] = b"finished-screen";
const NATURAL_EXIT_COMMAND: &[u8] = b"printf 'finished-screen\n'; exit 7\n";
const NATURAL_EXIT_CODE: i32 = 7;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reboot_reconciliation_never_claims_a_dead_runtime_was_reattached() {
    let mut harness = Harness::start().await;
    let mut sessions = Vec::new();
    for name in ["first-reboot", "second-reboot", "third-reboot"] {
        sessions.push(harness.create_cat(name).await);
    }
    let paths = harness.paths();
    let records = sessions
        .iter()
        .map(|session| {
            read_json::<RuntimeRecord>(&paths.runtime_record(session.manifest.terminal_id)).unwrap()
        })
        .collect::<Vec<_>>();
    simulate_power_loss(&mut harness, &records).await;
    for record in &records {
        assert_runtime_identities_gone(record);
        yolo_runtime::durable_reboot::adversarial::persist_previous_boot(
            &paths,
            record,
            PREVIOUS_BOOT_ID,
        )
        .unwrap();
    }
    harness.replace_host(spawn_host(harness.home.path())).await;
    let reconciliation = harness.client().reconcile().await.unwrap();
    assert_eq!(
        reconciliation.needs_recreate,
        records.len(),
        "Host stderr={}",
        harness.host_stderr_tail()
    );
    assert_eq!(reconciliation.reattached, usize::default());
    let lost = harness.client().list_sessions(false).await.unwrap();
    for listed in lost {
        assert_eq!(
            listed.manifest.recovery_state,
            yolo_runtime::RecoveryState::NeedsRecreate,
            "reconcile={reconciliation:?}; Host stderr={}",
            harness.host_stderr_tail()
        );
        let session = harness
            .client()
            .get_session(listed.manifest.terminal_id)
            .await
            .unwrap();
        let previous = records
            .iter()
            .find(|record| record.terminal_id == session.manifest.terminal_id)
            .expect("original reboot runtime");
        assert_eq!(
            session.manifest.recovery_state,
            yolo_runtime::RecoveryState::NeedsRecreate,
            "reconcile={reconciliation:?}; Host stderr={}",
            harness.host_stderr_tail()
        );
        assert_ne!(
            session.manifest.recovery_state,
            yolo_runtime::RecoveryState::Reattached
        );
        let lost_runtime = session.runtime.as_ref().expect("lost reboot runtime");
        assert_eq!(lost_runtime.runtime_epoch, previous.runtime_epoch);
        let selection = yolo_runtime::model::TerminalLifecycleSelection::selected(
            session.manifest.terminal_id,
            session.manifest.updated_at_ms,
            lost_runtime.status,
        )
        .with_expected_manifest_sha256(Some(
            yolo_runtime::model::TerminalManifestDigest::capture(&session.manifest).unwrap(),
        ))
        .with_expected_runtime_epoch(Some(lost_runtime.runtime_epoch));
        harness.client().reconcile().await.unwrap();
        let unchanged = harness
            .client()
            .get_session(session.manifest.terminal_id)
            .await
            .unwrap();
        assert_eq!(
            unchanged.manifest.updated_at_ms,
            session.manifest.updated_at_ms
        );
        assert_eq!(
            yolo_runtime::model::TerminalManifestDigest::capture(&unchanged.manifest).unwrap(),
            yolo_runtime::model::TerminalManifestDigest::capture(&session.manifest).unwrap(),
        );
        let recreated = harness
            .resume_with_preconditions_and_track(selection, true)
            .await
            .unwrap();
        assert_eq!(
            recreated.manifest.recovery_state,
            yolo_runtime::RecoveryState::Recreated
        );
        let runtime = recreated.runtime.as_ref().expect("recreated runtime");
        assert_eq!(runtime.status, TerminalRuntimeStatus::Running);
        assert_runtime_identity_rotated(previous, runtime);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn natural_exit_keeps_vt_snapshot_attachable_until_archive_retires_worker() {
    let harness = Harness::start().await;
    let created = harness
        .client()
        .create_session(&CreateTerminal {
            creation_token: Uuid::new_v4(),
            name: Some("completed-tui".into()),
            cwd: harness.home.path().to_path_buf(),
            cols: Some(80),
            rows: Some(24),
            initial_input: None,
            context: None,
        })
        .await
        .unwrap();
    let terminal_id = created.terminal_id;
    let view = harness.client().get_session(terminal_id).await.unwrap();
    harness.track_view(&view);
    let target = runtime_ref_from_view(&view);
    let lease = harness
        .client()
        .acquire_writer(target, "natural-exit", None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            NATURAL_EXIT_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    let runtime = view.runtime.as_ref().unwrap();
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    wait_for_exit(&harness.client(), terminal_id).await;
    wait_for_identity_gone(child).await;
    let exited = harness.client().get_session(terminal_id).await.unwrap();
    harness.track_view(&exited);
    assert!(worker.matches());
    let mut attachment = harness
        .client()
        .attach(
            terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let snapshot = attachment.initial.snapshot.as_ref().unwrap();
    assert!(
        snapshot
            .data
            .windows(NATURAL_EXIT_SCREEN.len())
            .any(|window| window == NATURAL_EXIT_SCREEN)
    );
    let event = attachment.next_event().await.unwrap();
    assert_eq!(event.event, event::SESSION_EXITED);
    let reported: ExitRecord = from_value(event.payload).unwrap();
    assert_eq!(reported.runtime_epoch, target.runtime_epoch);
    assert_eq!(reported.code, Some(NATURAL_EXIT_CODE));
    assert!(matches!(
        attachment.next_event().await,
        Err(ClientError::Protocol(ProtocolError::EndOfStream))
    ));
    let archived = harness.client().archive(terminal_id).await.unwrap();
    assert!(archived.manifest.archived);
    wait_for_identity_gone(worker).await;
}
