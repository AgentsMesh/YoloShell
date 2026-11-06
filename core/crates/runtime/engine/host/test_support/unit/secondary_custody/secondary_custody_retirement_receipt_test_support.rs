use super::*;
use crate::process::ProcessIdentity;
#[cfg(target_os = "macos")]
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
#[cfg(target_os = "macos")]
use crate::pty_custody::CustodyHolderIdentity;

pub(super) struct ReopenedRetirement {
    pub(super) host: Host,
    pub(super) runtime: RuntimeRecord,
    #[cfg(target_os = "macos")]
    guardian: CustodyHolderIdentity,
    #[cfg(target_os = "macos")]
    guardian_relation: Option<ProcessIdentityRelationGuard>,
    pub(super) _directory: HostTestDirectory,
}

impl ReopenedRetirement {
    pub(super) fn new() -> Self {
        let directory = tempdir().unwrap();
        let first = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let runtime = dead_runtime(&first, directory.path());
        first
            .secondary_custody
            .install_test_retirement_evidence(&first, &runtime);
        #[cfg(target_os = "macos")]
        let guardian = {
            let custody: CustodyDurableRecord = first
                .namespace
                .read_json(&first.paths.pty_custody(runtime.terminal_id))
                .unwrap();
            custody.retired_lease().unwrap().guardian.unwrap().process()
        };
        drop(first);
        let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            runtime,
            #[cfg(target_os = "macos")]
            guardian,
            #[cfg(target_os = "macos")]
            guardian_relation: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn new_with_guardian_relation(relation: ProcessIdentityRelation) -> Self {
        let mut fixture = Self::new();
        fixture.set_guardian_relation(relation);
        fixture
    }

    #[cfg(target_os = "macos")]
    pub(super) fn set_guardian_relation(&mut self, relation: ProcessIdentityRelation) {
        self.guardian_relation.take();
        self.guardian_relation = Some(crate::process::install_test_process_identity_relation(
            self.guardian.pid,
            self.guardian.start_time,
            relation,
        ));
    }
}

fn reaped_process_identity() -> ProcessIdentity {
    crate::host::reaped_host_test_process_identity()
}

pub(super) fn dead_runtime(host: &Host, home: &std::path::Path) -> RuntimeRecord {
    let mut record = runtime(host, home.join(RECEIPT_SOCKET_NAME));
    let worker = reaped_process_identity();
    let child = reaped_process_identity();
    record.worker_pid = worker.pid;
    record.worker_start_time = worker.start_time;
    record.child_pid = child.pid;
    record.child_start_time = child.start_time;
    record
}

pub(super) fn replace_file_same_value(path: &std::path::Path) {
    const REPLACEMENT_EXTENSION: &str = "same-value-replacement";
    let replacement = path.with_extension(REPLACEMENT_EXTENSION);
    std::fs::copy(path, &replacement).unwrap();
    std::fs::rename(replacement, path).unwrap();
}
