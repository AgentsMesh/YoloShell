use crate::host::tests::*;
use super::*;

const LINUX_RETIREMENT_FIXTURE_SOCKET: &str = "linux-retirement-fixture.sock";

#[test]
fn linux_retirement_fixture_does_not_authorize_live_runtime_identities() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let live = process::ProcessIdentity::capture(std::process::id()).unwrap();
    for (worker_live, child_live) in [(true, true), (true, false), (false, true)] {
        let manifest = fixture_manifest(directory.path());
        let mut record = dead_resume_runtime(&host, &manifest);
        if worker_live {
            (record.worker_pid, record.worker_start_time) = (live.pid, live.start_time);
        }
        if child_live {
            (record.child_pid, record.child_start_time) = (live.pid, live.start_time);
        }
        persist_host_session(&host, &manifest, &record);
        crate::host::linux_retirement_evidence::publish(&host, &record);
        let path = process::linux_containment::record_path(
            &host.paths.session_dir(record.terminal_id),
            record.runtime_epoch,
        );
        assert!(!path.exists());
    }
}

#[test]
fn linux_retirement_fixture_preserves_real_live_containment_until_child_exit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut worker = ArchiveTestProcess::spawn();
    let mut child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(LINUX_RETIREMENT_FIXTURE_SOCKET),
    );
    bind_archive_processes(&mut record, &worker, &child);
    persist_host_session(&host, &manifest, &record);
    let containment = crate::host::linux_retirement_evidence::capture_live(&host, &record);
    let path = process::linux_containment::record_path(
        &host.paths.session_dir(record.terminal_id),
        record.runtime_epoch,
    );
    let evidence: process::linux_containment::LinuxContainmentRecord = read_json(&path).unwrap();
    let leaf = evidence.delegated_root.join(&evidence.leaf_name);
    assert!(leaf.is_dir());
    assert_eq!(evidence.bound_pid, Some(record.child_pid));
    assert_eq!(evidence.bound_start_time, Some(record.child_start_time));
    let retained = std::fs::read(&path).unwrap();
    crate::host::linux_retirement_evidence::publish(&host, &record);
    assert_eq!(std::fs::read(&path).unwrap(), retained);
    assert!(leaf.is_dir());
    worker.stop();
    child.stop();
    drop(containment);
    assert!(!leaf.exists());
    crate::host::linux_retirement_evidence::publish(&host, &record);
    assert_eq!(std::fs::read(&path).unwrap(), retained);
}
