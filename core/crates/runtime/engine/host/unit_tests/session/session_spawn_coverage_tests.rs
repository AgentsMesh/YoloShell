use crate::host::tests::*;
use super::*;

pub(super) const MISSING_WORKER_DETAIL: &str = "worker did not become ready before timeout";
pub(super) const DEAD_PENDING_DETAIL: &str =
    "worker bootstrap remained pending after a process identity exited";
const LIVE_PENDING_DETAIL: &str = "worker bootstrap remained pending before timeout";
const UNEXPECTED_READY_DETAIL: &str = "timeout classification cannot report ready";

pub(super) fn uncertain_start_detail(start: WorkerStart) -> String {
    match start {
        WorkerStart::Uncertain(error) => error.to_string(),
        WorkerStart::Ready => panic!("{UNEXPECTED_READY_DETAIL}"),
    }
}

#[test]
fn worker_start_timeout_classifies_missing_dead_and_live_pending_evidence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);

    assert_eq!(
        uncertain_start_detail(
            host.classify_worker_start_timeout(&manifest, launch.runtime_epoch, launch.worker_id,)
                .unwrap(),
        ),
        MISSING_WORKER_DETAIL
    );

    let dead = runtime_for_creation(&host, &launch, BootstrapDelivery::Pending, false);
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &dead).unwrap();
    assert_eq!(
        uncertain_start_detail(
            host.classify_worker_start_timeout(&manifest, launch.runtime_epoch, launch.worker_id,)
                .unwrap(),
        ),
        DEAD_PENDING_DETAIL
    );

    let live = runtime_for_creation(&host, &launch, BootstrapDelivery::Pending, true);
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &live).unwrap();
    assert_eq!(
        uncertain_start_detail(
            host.classify_worker_start_timeout(&manifest, launch.runtime_epoch, launch.worker_id,)
                .unwrap(),
        ),
        LIVE_PENDING_DETAIL
    );
}

#[test]
fn planned_launch_and_unstarted_policy_preserve_exact_custody() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let planned = CreationLaunchIdentity::new();
    let launch = host
        .prepare_worker_launch(
            &manifest,
            shell_command(&manifest.cwd),
            None,
            Some(&planned),
        )
        .unwrap();
    assert_eq!(launch.runtime_epoch, planned.runtime_epoch);
    assert_eq!(launch.worker_id, planned.worker_id);
    assert_eq!(launch.auth_token, planned.auth_token);

    host.publish_worker_launch(&launch).unwrap();
    let path = host.paths.worker_launch(manifest.terminal_id);
    assert!(path.try_exists().unwrap());
    host.apply_unstarted_launch_policy(manifest.terminal_id, UnstartedLaunchPolicy::Preserve)
        .unwrap();
    assert!(path.try_exists().unwrap());
    host.apply_unstarted_launch_policy(manifest.terminal_id, UnstartedLaunchPolicy::Retire)
        .unwrap();
    assert!(!path.try_exists().unwrap());
    host.apply_unstarted_launch_policy(manifest.terminal_id, UnstartedLaunchPolicy::Retire)
        .unwrap();
    assert!(!path.try_exists().unwrap());
}
