use super::*;

const INITIAL_OUTPUT_OFFSET: u64 = 0;
const OBSERVATION_TIMEOUT_DETAIL: &str = "Host did not publish exact active custody evidence";
const HOST_EXIT_TIMEOUT_DETAIL: &str = "Host did not fail-stop after exact guardian death";
const PROCESS_GONE_TIMEOUT_DETAIL: &str = "signaled process identity did not become gone";

pub(super) async fn wait_for_observation(
    harness: &Harness,
    terminal_id: Uuid,
    generation: u64,
) -> ActiveCustodyObservation {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        if let Ok(observation) = ActiveCustodyObservation::read(&harness.paths(), terminal_id) {
            if observation.controller_generation() == generation {
                return observation;
            }
        }
        assert_before(deadline, OBSERVATION_TIMEOUT_DETAIL, harness);
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_host_fail_stop(harness: &mut Harness) -> std::process::ExitStatus {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        if let Some(status) = harness.try_wait_host().unwrap() {
            return status;
        }
        assert_before(deadline, HOST_EXIT_TIMEOUT_DETAIL, harness);
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_successor(
    harness: &Harness,
    terminal_id: Uuid,
    before: &RuntimeRecord,
) -> (RuntimeRecord, ActiveCustodyObservation) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        harness.client().reconcile().await.unwrap();
        let record: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
        if record.controller_generation > before.controller_generation {
            if let Ok(observation) = ActiveCustodyObservation::read(&harness.paths(), terminal_id) {
                if observation.controller_generation() == record.controller_generation {
                    return (record, observation);
                }
            }
        }
        assert_before(deadline, OBSERVATION_TIMEOUT_DETAIL, harness);
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_gone(identity: ProcessIdentity) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while identity.observe() != process::ProcessIdentityObservation::Gone {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{PROCESS_GONE_TIMEOUT_DETAIL}: {identity:?}"
        );
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

pub(super) fn assert_activation(
    observation: &ActiveCustodyObservation,
    runtime: &RuntimeRecord,
    host: ProcessIdentity,
) {
    assert_eq!(observation.runtime_epoch(), runtime.runtime_epoch);
    assert_eq!(observation.worker_id(), runtime.worker_id);
    assert_eq!(
        observation.controller_generation(),
        runtime.controller_generation
    );
    assert_eq!(local_identity(observation.host()), host);
    assert!(local_identity(observation.guardian()).matches());
}

pub(super) async fn assert_terminal_round_trip(
    harness: &Harness,
    runtime: &RuntimeRecord,
    holder: &str,
    input: &[u8],
    output: &[u8],
) {
    let target = runtime_ref_from_record(runtime);
    let attachment = harness
        .client()
        .attach(runtime.terminal_id, INITIAL_OUTPUT_OFFSET)
        .await
        .unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, holder, None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            input.to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, output).await;
}

pub(super) async fn assert_terminal_takeover_round_trip(
    harness: &Harness,
    runtime: &RuntimeRecord,
    holder: &str,
    input: &[u8],
    output: &[u8],
) {
    let target = runtime_ref_from_record(runtime);
    let attachment = harness
        .client()
        .attach(runtime.terminal_id, INITIAL_OUTPUT_OFFSET)
        .await
        .unwrap();
    let client = harness.client();
    let lease = match client.get_writer_lease(target).await.unwrap() {
        Some(observed) => {
            client
                .take_over_writer_for_instance(
                    target,
                    holder,
                    None,
                    Some(observed.generation),
                    WriterCapability::generate(),
                    None,
                )
                .await
        }
        None => client.acquire_writer(target, holder, None).await,
    }
    .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            input.to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, output).await;
}

pub(super) fn assert_same_runtime(actual: &RuntimeRecord, expected: &RuntimeRecord) {
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.worker_id, expected.worker_id);
    assert_eq!(runtime_process(actual), runtime_process(expected));
    assert_eq!(child_process(actual), child_process(expected));
    assert_eq!(actual.socket_path, expected.socket_path);
}

pub(super) fn runtime_process(runtime: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    }
}

pub(super) fn child_process(runtime: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    }
}

pub(super) fn local_identity(
    identity: yolo_process_test_support::ProcessIdentity,
) -> ProcessIdentity {
    ProcessIdentity {
        pid: identity.pid,
        start_time: identity.start_time,
    }
}

fn assert_before(deadline: tokio::time::Instant, detail: &str, harness: &Harness) {
    assert!(
        tokio::time::Instant::now() < deadline,
        "{detail}: {}",
        harness.host_stderr_tail()
    );
}
