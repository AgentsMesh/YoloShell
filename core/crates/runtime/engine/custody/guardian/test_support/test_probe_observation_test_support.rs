use super::super::*;
use super::test_record_support::{CONTROLLER_GENERATION, armed_record};
use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{ProcessIdentity, RetainedPtyProbeFixture};
use crate::pty_custody::CustodyHolderIdentity;
use portable_pty::PtySize;
use std::process::Command;
use std::time::{Duration, Instant};

const CAPABILITY: &str = "guardian-set-probe-capability";
const NAMESPACE_LABEL: &str = "guardian set probe test session";
const HOLDER_EXECUTABLE: &str = "/bin/sleep";
const HOLDER_ARGUMENT: &str = "60";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const CHILD_EXIT_POLL: Duration = Duration::from_millis(10);
const CHILD_EXIT_TIMEOUT_DETAIL: &str = "guardian probe test child cleanup timed out";

pub(super) struct ProbeFixture {
    _root: tempfile::TempDir,
    pub(super) paths: Paths,
    pub(super) namespace: NamespaceLease,
    pub(super) record: CustodyDurableRecord,
    pub(super) identity: FileIdentity,
    pub(super) probe: PtyMasterFdCustodyProbe,
    passive: RetainedPtyProbeFixture,
}

impl ProbeFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        let guardian = CustodyHolderIdentity::capture().unwrap();
        let record = armed_record(guardian);
        paths.ensure_session(record.runtime.terminal_id).unwrap();
        let session_dir = paths.session_dir(record.runtime.terminal_id);
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session_dir)]).unwrap();
        namespace
            .write_json(&paths.pty_custody(record.runtime.terminal_id), &record)
            .unwrap();
        let identity = namespace
            .read_json_with_identity::<CustodyDurableRecord>(
                &paths.pty_custody(record.runtime.terminal_id),
            )
            .unwrap()
            .1;
        let (passive, probe) = retained_probe();
        let publisher = record.armed_lease().unwrap().guardian.unwrap();
        publish_armed(
            &namespace,
            &paths,
            &record,
            identity,
            publisher,
            probe.clone(),
        )
        .unwrap();
        Self {
            _root: root,
            paths,
            namespace,
            record,
            identity,
            probe,
            passive,
        }
    }

    pub(super) fn stale_probe(
        &self,
        guardian_id: Uuid,
        identity: ProcessIdentity,
        custody: PtyMasterFdCustodyProbe,
    ) -> GuardianCustodyProbe {
        GuardianCustodyProbe {
            binding: ProbeBinding {
                schema_version: PROBE_SCHEMA_VERSION,
                guardian_id,
                guardian_process: holder(identity),
                controller_generation: CONTROLLER_GENERATION,
                durable_record_identity: self.identity,
            },
            custody,
        }
    }

    pub(super) fn write_probe(&self, probe: GuardianCustodyProbe) {
        self.namespace
            .write_json(
                &path(
                    &self.paths,
                    self.record.runtime.terminal_id,
                    probe.binding.guardian_id,
                ),
                &probe,
            )
            .unwrap();
    }

    pub(super) fn spawn_holder(&self) -> HolderChild {
        HolderChild::spawn(&self.passive)
    }
}

pub(super) struct HolderChild {
    _child: GuardedChild,
    pub(super) identity: ProcessIdentity,
}

impl HolderChild {
    fn spawn(passive: &RetainedPtyProbeFixture) -> Self {
        let mut command = Command::new(HOLDER_EXECUTABLE);
        command.arg(HOLDER_ARGUMENT);
        passive.configure_exec_inheritance(&mut command);
        let child = GuardedChild::spawn(|| crate::process::spawn_waitable_test_child(command));
        let identity = child.identity();
        Self {
            _child: child,
            identity,
        }
    }
}

pub(super) fn gone_identity() -> ProcessIdentity {
    let mut command = Command::new(HOLDER_EXECUTABLE);
    command.arg(HOLDER_ARGUMENT);
    let mut child = GuardedChild::spawn(|| crate::process::spawn_waitable_test_child(command));
    let identity = child.identity();
    child.request_cleanup().unwrap();
    let deadline = Instant::now() + CHILD_EXIT_TIMEOUT;
    assert!(
        child
            .wait_before(deadline, CHILD_EXIT_POLL)
            .unwrap()
            .is_some(),
        "{CHILD_EXIT_TIMEOUT_DETAIL}"
    );
    identity
}

fn retained_probe() -> (RetainedPtyProbeFixture, PtyMasterFdCustodyProbe) {
    RetainedPtyProbeFixture::new(
        PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        },
        CAPABILITY,
    )
}

fn holder(identity: ProcessIdentity) -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: identity.pid,
        start_time: identity.start_time,
    }
}
