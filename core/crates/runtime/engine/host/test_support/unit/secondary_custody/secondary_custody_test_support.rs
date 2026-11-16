use super::*;
use crate::process::{
    ActiveMacosSecondaryCustodyFixture, MacosSecondaryCustodyExport,
    MacosSecondaryCustodyFixtureAliases,
};
use portable_pty::PtySize;

#[cfg(test)]
#[path = "secondary_custody_runtime_child_test.rs"]
mod runtime_child;

const TEST_CAPABILITY: &str = "host-secondary-custody-test";
const TEST_PIXEL_SIZE: u16 = 0;
const GUARDIAN_CLEANUP_DETAIL: &str = "clean up exact test custody guardian processes";

enum ChildIdentityFixture {
    Spawned,
    Current,
    Provided(crate::process::ProcessIdentity),
}

pub(super) struct MacosHandshakeFixture {
    pub(super) host: Host,
    pub(super) record: RuntimeRecord,
    listener: Option<UnixListener>,
    export: Option<MacosSecondaryCustodyExport>,
    pending: Option<CustodyDurableRecord>,
    custody_path: std::path::PathBuf,
    aliases: Option<MacosSecondaryCustodyFixtureAliases>,
    runtime_worker: Option<runtime_child::TestRuntimeChild>,
    runtime_child: Option<runtime_child::TestRuntimeChild>,
    _directory: HostTestDirectory,
}

pub(super) struct MacosWorkerAliases {
    aliases: Option<MacosSecondaryCustodyFixtureAliases>,
    runtime_worker: Option<runtime_child::TestRuntimeChild>,
    runtime_child: Option<runtime_child::TestRuntimeChild>,
}

impl MacosHandshakeFixture {
    pub(super) fn new(terminal_id: Option<Uuid>) -> Self {
        Self::with_child_identity(terminal_id, ChildIdentityFixture::Spawned)
    }

    pub(super) fn with_live_child(terminal_id: Option<Uuid>) -> Self {
        Self::with_child_identity(terminal_id, ChildIdentityFixture::Current)
    }

    pub(super) fn with_exact_child(
        terminal_id: Option<Uuid>,
        child: crate::process::ProcessIdentity,
    ) -> Self {
        Self::with_child_identity(terminal_id, ChildIdentityFixture::Provided(child))
    }

    fn with_child_identity(terminal_id: Option<Uuid>, child_identity: ChildIdentityFixture) -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(super::super::super::HostConfig::new(directory.path())).unwrap();
        let socket = directory.path().join(TEST_SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let runtime_worker = runtime_child::TestRuntimeChild::spawn();
        let worker = runtime_worker.identity();
        let (runtime_child, child) = match child_identity {
            ChildIdentityFixture::Spawned => {
                let runtime_child = runtime_child::TestRuntimeChild::spawn();
                let child = runtime_child.identity();
                (Some(runtime_child), child)
            }
            ChildIdentityFixture::Current => (
                None,
                crate::process::ProcessIdentity::capture(std::process::id()).unwrap(),
            ),
            ChildIdentityFixture::Provided(child) => (None, child),
        };
        let mut record = runtime(&host, socket);
        if let Some(terminal_id) = terminal_id {
            record.terminal_id = terminal_id;
        }
        record.worker_pid = worker.pid;
        record.worker_start_time = worker.start_time;
        record.child_pid = child.pid;
        record.child_start_time = child.start_time;
        let pending = persist_handshake_fixture(&host, &record);
        let custody_path = host.paths.pty_custody(record.terminal_id);
        let active = ActiveMacosSecondaryCustodyFixture::new(
            PtySize {
                rows: TEST_ROWS,
                cols: TEST_COLUMNS,
                pixel_width: TEST_PIXEL_SIZE,
                pixel_height: TEST_PIXEL_SIZE,
            },
            TEST_CAPABILITY,
        );
        let runtime = CustodyRuntimeIdentity::from_record(&record);
        let (aliases, export) = active.export(&runtime);
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            record,
            listener: Some(listener),
            export: Some(export),
            pending: Some(pending),
            custody_path,
            aliases: Some(aliases),
            runtime_worker: Some(runtime_worker),
            runtime_child,
        }
    }

    pub(super) fn close_worker_aliases(&mut self) {
        self.settle_runtime_worker();
        self.settle_runtime_child();
        drop(self.aliases.take());
    }

    pub(super) fn take_worker_aliases(&mut self) -> MacosWorkerAliases {
        MacosWorkerAliases {
            aliases: self.aliases.take(),
            runtime_worker: self.runtime_worker.take(),
            runtime_child: self.runtime_child.take(),
        }
    }

    fn settle_runtime_worker(&mut self) {
        if let Some(worker) = self.runtime_worker.as_mut() {
            worker.settle();
        }
    }

    pub(super) fn settle_runtime_child(&mut self) {
        if let Some(child) = self.runtime_child.as_mut() {
            child.settle();
        }
    }

    pub(super) fn capability_writer_handle(&self) -> u64 {
        self.aliases.as_ref().unwrap().writer_handle()
    }

    pub(super) fn publish_for_host(&mut self, host: &Host) {
        self.pending = Some(persist_successor_handshake_fixture(host, &self.record));
        self.custody_path = host.paths.pty_custody(self.record.terminal_id);
    }
}

impl MacosWorkerAliases {
    pub(super) fn close(mut self) {
        if let Some(worker) = self.runtime_worker.as_mut() {
            worker.settle();
        }
        if let Some(child) = self.runtime_child.as_mut() {
            child.settle();
        }
        drop(self.aliases.take());
    }
}

impl Drop for MacosWorkerAliases {
    fn drop(&mut self) {
        if let Some(worker) = self.runtime_worker.as_mut() {
            worker.settle();
        }
        if let Some(child) = self.runtime_child.as_mut() {
            child.settle();
        }
    }
}

impl Drop for MacosHandshakeFixture {
    fn drop(&mut self) {
        self.settle_runtime_worker();
        self.settle_runtime_child();
        if let Err(error) =
            crate::custody_guardian::terminate_test_processes_for_home(self.host.paths.home())
        {
            if std::thread::panicking() {
                eprintln!("{GUARDIAN_CLEANUP_DETAIL}: {error:#}");
            } else {
                panic!("{GUARDIAN_CLEANUP_DETAIL}: {error:#}");
            }
        }
    }
}

include!("secondary_custody_test_fixture_behaviors.rs");
include!("secondary_custody_premature_release_fixture.rs");
include!("secondary_custody_rollback_test_fixture.rs");
