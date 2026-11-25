use crate::worker::tests::*;
const SURVIVOR_TEST_HOLDER: &str = "startup-survivor";
const SURVIVOR_RELEASE_INPUT: &[u8] = b"release\n";
const SURVIVOR_OUTPUT: &[u8] = b"survivor-ready";
const SURVIVOR_RELEASE_ACK: &[u8] = b"survivor-release-ack";
const SURVIVOR_ATTACH_AFTER_SEQUENCE: u64 = 0;
const SURVIVOR_NO_RESPONSE_DETAIL: &str = "no worker response";
const SURVIVOR_MARKER_FILE: &str = "bootstrap-child.marker";
const SURVIVOR_EXIT_MARKER_FILE: &str = "bootstrap-child-exiting.marker";
const SURVIVOR_EARLY_EXIT_DETAIL: &str = "worker exited after publishing Survivor";
const SURVIVOR_STARTUP_LIFECYCLE_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("startup_bootstrap_process_fixture.rs");
include!("../../test_support/integration/startup_bootstrap/startup_bootstrap_diagnostic.rs");

async fn wait_for_survivor_record(paths: &Paths, terminal_id: Uuid, marker: &Path) -> RuntimeRecord {
    tokio::time::timeout(SURVIVOR_STARTUP_LIFECYCLE_TIMEOUT, async {
        loop {
            if let Ok(record) = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
                && record.bootstrap_delivery == BootstrapDelivery::Survivor
                && marker.exists()
            {
                return record;
            }
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap_or_else(|error| {
        panic!(
            "{error}: {}",
            survivor_startup_diagnostic(paths, terminal_id)
        )
    })
}

async fn wait_for_worker_status(record: &RuntimeRecord, expected: TerminalRuntimeStatus) -> RuntimeView {
    let timeout = match expected {
        TerminalRuntimeStatus::Exited => WORKER_TERMINATION_TIMEOUT,
        _ => WORKER_AUTH_TIMEOUT,
    };
    let mut observed = String::from(SURVIVOR_NO_RESPONSE_DETAIL);
    tokio::time::timeout(timeout, async {
        loop {
            match rpc(
                record,
                record.controller_generation,
                WORKER_STATUS_METHOD,
                &(),
            )
            .await
            {
                Ok(response) if response.ok => {
                    let status: RuntimeView = response.decode_payload().unwrap();
                    observed = format!("status {:?}", status.status);
                    if status.status == expected {
                        return status;
                    }
                }
                Ok(response) => observed = format!("error response {:?}", response.error),
                Err(error) => observed = error.to_string(),
            }
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("expected {expected:?}, last observed {observed}"))
}

async fn shutdown_exited_worker(record: &RuntimeRecord) {
    let response = rpc(
        record,
        record.controller_generation,
        WORKER_SHUTDOWN_METHOD,
        &TerminalRef {
            terminal_id: record.terminal_id,
        },
    )
    .await
    .unwrap();
    assert!(response.ok);
}

async fn wait_for_survivor_output(record: &RuntimeRecord, expected: &[u8]) -> AttachResult {
    tokio::time::timeout(WORKER_AUTH_TIMEOUT, async {
        loop {
            if let Ok(response) = rpc(
                record,
                record.controller_generation,
                crate::protocol::method::ATTACH_SESSION,
                &AttachRequest {
                    terminal_id: record.terminal_id,
                    after_seq: SURVIVOR_ATTACH_AFTER_SEQUENCE,
                },
            )
            .await
                && response.ok
            {
                let attached: AttachResult = response.decode_payload().unwrap();
                if attachment_contains(&attached, expected) {
                    return attached;
                }
            }
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn failed_bootstrap_promotes_an_authenticated_usable_survivor() {
    let directory = GuardianRuntimeDirectory::new();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let marker = directory.path().join(SURVIVOR_MARKER_FILE);
    let exit_marker = directory.path().join(SURVIVOR_EXIT_MARKER_FILE);
    let mut launch = adversarial_launch(&paths, terminal_id, &marker);
    configure_survivor_launch(&mut launch, &marker, &exit_marker);
    write_launch(&paths, terminal_id, &launch);
    write_adversarial_manifest(&paths, terminal_id);

    let custody = spawn_active_runtime_custody(paths.clone(), terminal_id);
    runtime_result_test_hook::watch(terminal_id);
    let worker = SurvivorWorkerTask::spawn(paths.clone(), terminal_id);
    let record = wait_for_survivor_record(&paths, terminal_id, &marker).await;
    let child = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    let mut cleanup = TestProcessCleanup(Some(child));
    assert!(!worker.is_finished(), "{SURVIVOR_EARLY_EXIT_DETAIL}");
    let status = wait_for_worker_status(&record, TerminalRuntimeStatus::Running).await;
    assert_eq!(status.bootstrap_delivery, BootstrapDelivery::Survivor);
    assert_eq!(std::fs::read(&marker).unwrap(), TEST_MARKER_CONTENT);
    assert_eq!(
        child.observe(),
        process::ProcessIdentityObservation::Matching
    );
    let attached = wait_for_survivor_output(&record, SURVIVOR_OUTPUT).await;
    assert_eq!(attached.runtime_epoch, record.runtime_epoch);

    let acquired = rpc(
        &record,
        record.controller_generation,
        crate::protocol::method::ACQUIRE_WRITER,
        &LeaseRequest {
            terminal_id,
            runtime_epoch: record.runtime_epoch,
            holder_id: SURVIVOR_TEST_HOLDER.into(),
            ttl_ms: None,
            generation: None,
            controller_instance_id: None,
            writer_capability: WriterCapability::generate(),
        },
    )
    .await
    .unwrap();
    assert!(acquired.ok);
    let lease: WriterLeaseGrant = acquired.decode_payload().unwrap();
    let delivered = rpc(
        &record,
        record.controller_generation,
        crate::protocol::method::TERMINAL_INPUT,
        &InputRequest {
            terminal_id,
            runtime_epoch: record.runtime_epoch,
            generation: lease.generation,
            writer_capability: lease.writer_capability,
            input_id: Uuid::new_v4(),
            data: SURVIVOR_RELEASE_INPUT.to_vec(),
        },
    )
    .await
    .unwrap();
    assert!(delivered.ok);
    wait_for_survivor_output(&record, SURVIVOR_RELEASE_ACK).await;
    assert_eq!(std::fs::read(exit_marker).unwrap(), TEST_MARKER_CONTENT);
    wait_for_process_exit(child).await;
    wait_for_worker_status(&record, TerminalRuntimeStatus::Exited).await;
    cleanup.0 = None;
    shutdown_exited_worker(&record).await;
    worker.finish().await;
    custody.await.unwrap().unwrap();
    assert!(!paths.worker_launch(terminal_id).exists());
    assert!(!paths.worker_socket(terminal_id).unwrap().exists());
    directory.close(&paths).unwrap();
}
