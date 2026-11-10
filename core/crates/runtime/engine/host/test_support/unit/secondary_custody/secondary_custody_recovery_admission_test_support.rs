use super::*;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};

const TEST_SOCKET_NAME: &str = "recovery-admission.sock";
const TEST_AUTH_TOKEN: &str = "recovery-admission-token";
const TEST_READER_HANDLE: u64 = 0x81;
const TEST_WRITER_HANDLE: u64 = 0x82;
const TEST_DIGEST_BYTE: u8 = 0x83;
const TEST_GUARDIAN_PID: u32 = 47_001;
const TEST_GUARDIAN_START_TIME: u64 = 57_001;
const TEST_DEAD_PROCESS_PID: u32 = u32::MAX;
const TEST_DEAD_PROCESS_START_TIME: u64 = 1;

#[derive(Clone, Copy)]
pub(super) enum FenceState {
    PreparedGuardian,
    ArmedSuccessor,
}

pub(super) struct RecoveryAdmissionFixture {
    _guardian_relation: ProcessIdentityRelationGuard,
    host: Host,
    runtime: CustodyRuntimeIdentity,
    _directory: HostTestDirectory,
}

impl RecoveryAdmissionFixture {
    pub(super) fn new(state: FenceState, relation: ProcessIdentityRelation) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let first = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let mut runtime = super::super::runtime(&first, directory.path().join(TEST_SOCKET_NAME));
        runtime.auth_token = TEST_AUTH_TOKEN.to_owned();
        runtime.worker_pid = TEST_DEAD_PROCESS_PID;
        runtime.worker_start_time = TEST_DEAD_PROCESS_START_TIME;
        runtime.child_pid = TEST_DEAD_PROCESS_PID;
        runtime.child_start_time = TEST_DEAD_PROCESS_START_TIME;
        first.ensure_session_namespace(runtime.terminal_id).unwrap();
        first
            .namespace
            .write_json(&first.paths.runtime_record(runtime.terminal_id), &runtime)
            .unwrap();
        let runtime_identity = CustodyRuntimeIdentity::from_record(&runtime);
        let guardian = CustodyHolderIdentity {
            pid: TEST_GUARDIAN_PID,
            start_time: TEST_GUARDIAN_START_TIME,
        };
        let predecessor = initial_guarded_record(
            &first,
            runtime_identity.clone(),
            runtime.controller_generation,
            guardian,
        );
        first
            .namespace
            .write_json(&first.paths.pty_custody(runtime.terminal_id), &predecessor)
            .unwrap();
        drop(first);

        let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let predecessor_binding = predecessor.armed_lease().unwrap().guardian.unwrap();
        let successor_request =
            request(&host, runtime_identity.clone(), host.controller_generation);
        let successor_offer = offer(&successor_request);
        let successor_binding = CustodyGuardianBinding::new(
            predecessor_binding.guardian_id(),
            guardian,
            &successor_request,
            &successor_offer,
        )
        .unwrap();
        let prepared = predecessor
            .prepare_guardian(&successor_request, &successor_offer, successor_binding)
            .unwrap();
        let installed = match state {
            FenceState::PreparedGuardian => prepared,
            FenceState::ArmedSuccessor => prepared
                .arm_prepared_guardian(&successor_request, &successor_offer, successor_binding)
                .unwrap(),
        };
        host.namespace
            .write_json(&host.paths.pty_custody(runtime.terminal_id), &installed)
            .unwrap();
        let guardian_relation = crate::process::install_test_process_identity_relation(
            guardian.pid,
            guardian.start_time,
            relation,
        );
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            _guardian_relation: guardian_relation,
            host,
            runtime: runtime_identity,
        }
    }

    pub(super) fn admission(&self) -> SecondaryCustodyRecoveryAdmission {
        self.host
            .secondary_custody
            .recovery_admission(&self.host, &self.runtime)
    }
}

fn initial_guarded_record(
    host: &Host,
    runtime: CustodyRuntimeIdentity,
    generation: u64,
    guardian: CustodyHolderIdentity,
) -> CustodyDurableRecord {
    let request = request(host, runtime.clone(), generation);
    let offer = offer(&request);
    let binding =
        CustodyGuardianBinding::new(uuid::Uuid::new_v4(), guardian, &request, &offer).unwrap();
    CustodyDurableRecord::pending(runtime)
        .prepare_guardian(&request, &offer, binding)
        .unwrap()
        .arm_prepared_guardian(&request, &offer, binding)
        .unwrap()
}

fn request(host: &Host, runtime: CustodyRuntimeIdentity, generation: u64) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime,
        host.secondary_custody_holder,
        TEST_AUTH_TOKEN.to_owned(),
        generation,
    )
}

fn offer(request: &CustodyRequest) -> CustodyOffer {
    crate::host::test_custody_offer(
        request,
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE),
        crate::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE),
    )
}
