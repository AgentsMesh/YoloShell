use super::{Host, RuntimeRecord};
use crate::process::linux_containment::{
    LinuxContainmentBroker, LinuxContainmentError, LinuxContainmentLease, LinuxContainmentRecord,
    record_path,
};
use crate::process::{
    BootIdentityRelation, ProcessIdentity, ProcessIdentityObservation, PtySessionProof,
};

const DELEGATION_REQUIRED: &str = "Linux Host lifecycle tests require a delegated cgroup root";
const TEST_PTY_COLUMNS: u16 = 2;
const TEST_PTY_ROWS: u16 = 2;
const SESSION_LABEL: &str = "Linux lifecycle fixture session";

pub(in crate::host) fn publish(host: &Host, runtime: &RuntimeRecord) {
    publish_paths(&host.paths, runtime);
}

pub(in crate::host) fn publish_paths(paths: &crate::paths::Paths, runtime: &RuntimeRecord) {
    if crate::process::boot_identity_relation(&runtime.boot_id, &crate::process::boot_id())
        != BootIdentityRelation::Same
        || !gone(runtime.worker_pid, runtime.worker_start_time)
        || !gone(runtime.child_pid, runtime.child_start_time)
    {
        return;
    }
    let session = paths.session_dir(runtime.terminal_id);
    let path = record_path(&session, runtime.runtime_epoch);
    let namespace =
        crate::namespace_lease::NamespaceLease::capture(&[(SESSION_LABEL, &session)]).unwrap();
    let root = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    if !namespace.file_is_missing(&path).unwrap() {
        let previous: LinuxContainmentRecord = namespace.read_json(&path).unwrap();
        assert!(
            previous
                .validate_recovery(
                    runtime.terminal_id,
                    runtime.runtime_epoch,
                    &runtime.boot_id,
                    root
                )
                .unwrap()
        );
        return;
    }
    let (lease, _pair) = prepare(runtime);
    let mut evidence = lease.record().clone();
    // These identities model an already-reaped runtime; no live process is migrated.
    evidence.bound_pid = Some(root.pid);
    evidence.bound_start_time = Some(root.start_time);
    lease.remove_empty().unwrap();
    assert!(
        evidence
            .validate_recovery(
                runtime.terminal_id,
                runtime.runtime_epoch,
                &runtime.boot_id,
                root
            )
            .unwrap()
    );
    namespace.write_json(&path, &evidence).unwrap();
}

pub(in crate::host) struct LiveFixtureContainment {
    lease: Option<LinuxContainmentLease>,
    root: ProcessIdentity,
    _pair: portable_pty::PtyPair,
}

pub(in crate::host) fn capture_live(
    host: &Host,
    runtime: &RuntimeRecord,
) -> LiveFixtureContainment {
    assert_eq!(runtime.boot_id, crate::process::boot_id());
    let root = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    assert_ne!(root.pid, std::process::id());
    assert_eq!(ProcessIdentity::capture(root.pid).unwrap(), root);
    let (mut lease, pair) = prepare(runtime);
    lease.migrate_bootstrap(root).unwrap();
    host.namespace
        .write_json(
            &record_path(
                &host.paths.session_dir(runtime.terminal_id),
                runtime.runtime_epoch,
            ),
            lease.record(),
        )
        .unwrap();
    LiveFixtureContainment {
        lease: Some(lease),
        root,
        _pair: pair,
    }
}

impl Drop for LiveFixtureContainment {
    fn drop(&mut self) {
        if !gone(self.root.pid, self.root.start_time) {
            return;
        }
        if let Some(lease) = self.lease.take() {
            match lease.remove_empty() {
                Ok(()) => {}
                Err(LinuxContainmentError::Io(error))
                    if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) if !std::thread::panicking() => panic!("{error}"),
                Err(_) => {}
            }
        }
    }
}

fn prepare(runtime: &RuntimeRecord) -> (LinuxContainmentLease, portable_pty::PtyPair) {
    let broker = LinuxContainmentBroker::discover()
        .unwrap()
        .expect(DELEGATION_REQUIRED);
    let mut lease = broker
        .prepare(runtime.terminal_id, runtime.runtime_epoch)
        .unwrap();
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            cols: TEST_PTY_COLUMNS,
            rows: TEST_PTY_ROWS,
            pixel_width: u16::default(),
            pixel_height: u16::default(),
        })
        .unwrap();
    let proof = PtySessionProof::capture(
        pair.master.tty_name().unwrap(),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    lease.bind_pty_proof(&proof).unwrap();
    (lease, pair)
}

fn gone(pid: u32, start_time: u64) -> bool {
    ProcessIdentity { pid, start_time }.observe() == ProcessIdentityObservation::Gone
}
