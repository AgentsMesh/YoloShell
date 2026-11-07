use crate::host::tests::*;
const SESSION_SPAWN_WAVE4_READY_TIMEOUT: Duration = Duration::from_millis(5);
const SESSION_SPAWN_WAVE4_POLL_INTERVAL: Duration = Duration::from_millis(1);
const SESSION_SPAWN_WAVE4_EXPECTED_ERROR: &str = "worker start unexpectedly succeeded";

#[tokio::test]
async fn published_worker_launch_rejects_another_session_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let mut launch = shell_resume_launch(&host, &manifest);
    launch.terminal_id = Uuid::new_v4();

    let error = session_spawn_wave4_error(
        host.spawn_published_worker(&manifest, &launch, UnstartedLaunchPolicy::Preserve)
            .await,
    );
    assert!(
        error
            .to_string()
            .contains(PUBLISHED_LAUNCH_SESSION_MISMATCH)
    );
}

#[derive(Clone, Copy, Debug)]
enum BootstrapStartWave4Case {
    Pending,
    Unknown,
    Failed,
    RuntimeEpoch,
    WorkerId,
    Authentication,
}

#[tokio::test(start_paused = true)]
async fn worker_start_wait_distinguishes_nonfinal_bootstrap_states() {
    for case in [
        BootstrapStartWave4Case::Pending,
        BootstrapStartWave4Case::Unknown,
        BootstrapStartWave4Case::Failed,
        BootstrapStartWave4Case::RuntimeEpoch,
        BootstrapStartWave4Case::WorkerId,
        BootstrapStartWave4Case::Authentication,
    ] {
        let directory = tempdir().unwrap();
        let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
        host.worker_ready_timeout = SESSION_SPAWN_WAVE4_READY_TIMEOUT;
        host.worker_ready_poll_interval = SESSION_SPAWN_WAVE4_POLL_INTERVAL;
        let manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let launch = shell_resume_launch(&host, &manifest);
        let delivery = match case {
            BootstrapStartWave4Case::Pending => BootstrapDelivery::Pending,
            BootstrapStartWave4Case::Unknown => BootstrapDelivery::Unknown,
            BootstrapStartWave4Case::Failed => BootstrapDelivery::Failed,
            BootstrapStartWave4Case::RuntimeEpoch | BootstrapStartWave4Case::WorkerId => {
                BootstrapDelivery::Pending
            }
            BootstrapStartWave4Case::Authentication => BootstrapDelivery::NotRequired,
        };
        let mut record = runtime_for_creation(&host, &launch, delivery, false);
        match case {
            BootstrapStartWave4Case::RuntimeEpoch => record.runtime_epoch = Uuid::new_v4(),
            BootstrapStartWave4Case::WorkerId => record.worker_id = Uuid::new_v4(),
            _ => {}
        }
        atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();

        let result = host
            .await_worker_start(&manifest, launch.runtime_epoch, launch.worker_id)
            .await;
        match case {
            BootstrapStartWave4Case::Pending => {
                assert_eq!(uncertain_start_detail(result.unwrap()), DEAD_PENDING_DETAIL)
            }
            BootstrapStartWave4Case::Unknown => {
                assert!(uncertain_start_detail(result.unwrap()).contains(WORKER_BOOTSTRAP_UNKNOWN))
            }
            BootstrapStartWave4Case::Failed => {
                assert!(
                    session_spawn_wave4_error(result)
                        .to_string()
                        .contains(WORKER_BOOTSTRAP_FAILED)
                )
            }
            BootstrapStartWave4Case::RuntimeEpoch
            | BootstrapStartWave4Case::WorkerId
            | BootstrapStartWave4Case::Authentication => {
                assert_eq!(
                    uncertain_start_detail(result.unwrap()),
                    MISSING_WORKER_DETAIL
                )
            }
        }
    }
}

pub(super) fn session_spawn_wave4_error(result: Result<WorkerStart>) -> anyhow::Error {
    match result {
        Err(error) => error,
        Ok(_) => panic!("{SESSION_SPAWN_WAVE4_EXPECTED_ERROR}"),
    }
}

#[derive(Clone, Copy, Debug)]
enum TimeoutIdentityWave4Case {
    RuntimeEpoch,
    WorkerId,
    Delivery,
}

#[test]
fn worker_start_timeout_rejects_each_nonmatching_predicate() {
    for case in [
        TimeoutIdentityWave4Case::RuntimeEpoch,
        TimeoutIdentityWave4Case::WorkerId,
        TimeoutIdentityWave4Case::Delivery,
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let launch = shell_resume_launch(&host, &manifest);
        let mut record = runtime_for_creation(&host, &launch, BootstrapDelivery::Pending, false);
        match case {
            TimeoutIdentityWave4Case::RuntimeEpoch => record.runtime_epoch = Uuid::new_v4(),
            TimeoutIdentityWave4Case::WorkerId => record.worker_id = Uuid::new_v4(),
            TimeoutIdentityWave4Case::Delivery => {
                record.bootstrap_delivery = BootstrapDelivery::NotRequired
            }
        }
        atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();

        let detail = uncertain_start_detail(
            host.classify_worker_start_timeout(&manifest, launch.runtime_epoch, launch.worker_id)
                .unwrap(),
        );
        assert_eq!(detail, MISSING_WORKER_DETAIL, "unexpected case: {case:?}");
    }
}
