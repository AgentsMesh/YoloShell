use super::*;
use crate::namespace_lease::NamespaceLease;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianBinding, CustodyGuardianPreparation,
    CustodyHolderIdentity, CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};

const NAMESPACE_LABEL: &str = "initial guardian settlement test session";
const AUTH_TOKEN: &str = "initial-guardian-settlement-token";
const BOOT_ID: &str = "initial-guardian-settlement-boot";
const CONTROLLER_GENERATION: u64 = 7;
const WORKER_PID: u32 = 49_001;
const WORKER_START: u64 = 59_001;
const CHILD_PID: u32 = 49_002;
const CHILD_START: u64 = 59_002;
const HOST_FIRST_PID: u32 = 49_003;
const HOST_UNKNOWN_PID: u32 = 49_004;
const HOST_ABA_PID: u32 = 49_005;
const HOST_START: u64 = 59_003;
const READER_HANDLE: u64 = 69_001;
const WRITER_HANDLE: u64 = 69_002;
const DIGEST_BYTE: u8 = 0x71;

#[test]
fn worker_first_death_waits_for_host_then_retains_initial_guardian() {
    let fixture = Fixture::new(HOST_FIRST_PID);
    let mut state = fixture.state();
    let host = relation(fixture.host, ProcessIdentityRelation::Matching);
    let _worker = relation(fixture.worker(), ProcessIdentityRelation::Gone);

    assert!(matches!(
        fixture.resolve(&mut state),
        Ok(Resolution::Pending)
    ));
    drop(host);
    let _host_gone = relation(fixture.host, ProcessIdentityRelation::Gone);
    assert!(matches!(
        fixture.resolve(&mut state),
        Ok(Resolution::InitialOrphan)
    ));
    assert_eq!(
        fixture.current(),
        (fixture.prepared.clone(), fixture.identity)
    );
}

#[test]
fn unknown_host_observation_keeps_exact_prepared_record() {
    let fixture = Fixture::new(HOST_UNKNOWN_PID);
    let mut state = fixture.state();
    let _host = relation(fixture.host, ProcessIdentityRelation::Unknown);

    assert!(fixture.resolve(&mut state).is_err());
    assert_eq!(
        fixture.current(),
        (fixture.prepared.clone(), fixture.identity)
    );
}

#[test]
fn same_value_inode_replacement_fails_closed_before_initial_orphan() {
    let fixture = Fixture::new(HOST_ABA_PID);
    let mut state = fixture.state();
    let _host = relation(fixture.host, ProcessIdentityRelation::Gone);
    crate::paths::atomic_write_json(&fixture.path, &fixture.prepared).unwrap();
    let replacement = fixture.current();
    assert_ne!(replacement.1, fixture.identity);

    assert!(
        resolve(
            &fixture.namespace,
            &fixture.paths,
            &mut state,
            &replacement.0,
            replacement.1,
        )
        .is_err()
    );
    assert_eq!(fixture.current(), replacement);
}

struct Fixture {
    _root: tempfile::TempDir,
    paths: Paths,
    namespace: NamespaceLease,
    path: std::path::PathBuf,
    prepared: CustodyDurableRecord,
    identity: FileIdentity,
    preparation: CustodyGuardianPreparation,
    host: CustodyHolderIdentity,
}

impl Fixture {
    fn new(host_pid: u32) -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        let runtime = runtime();
        paths.ensure_session(runtime.terminal_id).unwrap();
        let session = paths.session_dir(runtime.terminal_id);
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
        let host = holder(host_pid, HOST_START);
        let request = CustodyRequest::new(
            CustodyLeaseId::new(),
            runtime.clone(),
            host,
            AUTH_TOKEN.to_owned(),
            CONTROLLER_GENERATION,
        );
        let offer = CustodyOffer::new_with_profile(
            &request,
            CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
            crate::pty_custody::CustodyProfile::ActiveRunning,
            crate::sha256_digest::Sha256Digest::repeated(DIGEST_BYTE),
        )
        .unwrap();
        let guardian = CustodyGuardianBinding::new(
            uuid::Uuid::new_v4(),
            CustodyHolderIdentity::capture().unwrap(),
            &request,
            &offer,
        )
        .unwrap();
        let prepared = CustodyDurableRecord::pending(runtime)
            .prepare_guardian(&request, &offer, guardian)
            .unwrap();
        let path = paths.pty_custody(prepared.runtime.terminal_id);
        namespace.write_json(&path, &prepared).unwrap();
        let identity = namespace
            .read_json_with_identity::<CustodyDurableRecord>(&path)
            .unwrap()
            .1;
        let preparation = CustodyGuardianPreparation::new(guardian, identity, identity, identity);
        Self {
            _root: root,
            paths,
            namespace,
            path,
            prepared,
            identity,
            preparation,
            host,
        }
    }

    fn state(&self) -> GuardianState {
        GuardianState::new_prepared_fixture(
            self.preparation,
            self.prepared.clone(),
            crate::pty_custody::CustodyDurableSnapshot::new(
                CustodyDurableRecord::pending(self.prepared.runtime.clone()),
                self.preparation.predecessor_file_identity(),
            ),
            self.prepared.runtime.clone(),
        )
    }

    fn resolve(&self, state: &mut GuardianState) -> Result<Resolution> {
        resolve(
            &self.namespace,
            &self.paths,
            state,
            &self.prepared,
            self.identity,
        )
    }

    fn current(&self) -> (CustodyDurableRecord, FileIdentity) {
        self.namespace.read_json_with_identity(&self.path).unwrap()
    }

    fn worker(&self) -> CustodyHolderIdentity {
        holder(WORKER_PID, WORKER_START)
    }
}

fn runtime() -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: BOOT_ID.to_owned(),
    }
}

fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}

fn relation(
    holder: CustodyHolderIdentity,
    relation: ProcessIdentityRelation,
) -> ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(holder.pid, holder.start_time, relation)
}
