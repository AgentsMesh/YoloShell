use super::*;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableLease, CustodyDurableRecord, CustodyRequest,
};

#[cfg(target_os = "macos")]
const RETIRED_TEST_READER_HANDLE: u64 = 71;
#[cfg(target_os = "macos")]
const RETIRED_TEST_WRITER_HANDLE: u64 = 72;
const RETIRED_TEST_JOURNAL_DETAIL: &str = "prepare retired custody test journal evidence";

pub(super) struct PreparedRetirement {
    request: CustodyRequest,
    offer: CustodyOffer,
    durable: CustodyDurableLease,
}

impl PreparedRetirement {
    pub(super) fn prepare(host: &Host, record: &RuntimeRecord) -> Self {
        let request = CustodyRequest::new(
            crate::pty_custody::CustodyLeaseId::new(),
            CustodyRuntimeIdentity::from_record(record),
            host.secondary_custody_holder,
            record.auth_token.clone(),
            record.controller_generation,
        );
        let offer = crate::host::test_custody_offer(
            &request,
            platform_descriptors(),
            crate::sha256_digest::Sha256Digest::repeated(RETIRED_TEST_DIGEST_BYTE),
        );
        let durable = persist_armed(host, record, &request, &offer);
        Self {
            request,
            offer,
            durable,
        }
    }

    pub(super) fn request(&self) -> &CustodyRequest {
        &self.request
    }

    pub(super) fn offer(&self) -> &CustodyOffer {
        &self.offer
    }

    pub(super) fn publish(self, host: &Host, record: &RuntimeRecord) {
        let journal_path = host.paths.journal(record.terminal_id);
        drop(
            crate::journal::Journal::open(&journal_path, record.runtime_epoch)
                .expect(RETIRED_TEST_JOURNAL_DETAIL),
        );
        let (_, runtime_identity): (RuntimeRecord, FileIdentity) = host
            .namespace
            .read_json_with_identity(&host.paths.runtime_record(record.terminal_id))
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        let journal = crate::journal::Journal::inspect_retained_evidence(
            host.namespace
                .file(&journal_path)
                .expect(RETIRED_TEST_JOURNAL_DETAIL),
            record.runtime_epoch,
        )
        .expect(RETIRED_TEST_JOURNAL_DETAIL);
        let evidence = journal
            .custody_retirement_evidence(self.durable.controller_generation, runtime_identity);
        let custody_path = host.paths.pty_custody(record.terminal_id);
        let armed: CustodyDurableRecord = host
            .namespace
            .read_json(&custody_path)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        let receipt = armed
            .retire_with_evidence(self.durable, evidence)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        host.namespace
            .write_json(&custody_path, &receipt)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        #[cfg(target_os = "linux")]
        crate::host::linux_retirement_evidence::publish(host, record);
    }
}

fn persist_armed(
    host: &Host,
    record: &RuntimeRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) -> CustodyDurableLease {
    host.ensure_session_namespace(record.terminal_id)
        .expect(RETIRED_TEST_INSTALL_DETAIL);
    host.namespace
        .write_json(&host.paths.runtime_record(record.terminal_id), record)
        .expect(RETIRED_TEST_INSTALL_DETAIL);
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let armed = arm_platform(pending, request, offer);
    host.namespace
        .write_json(&host.paths.pty_custody(record.terminal_id), &armed)
        .expect(RETIRED_TEST_INSTALL_DETAIL);
    armed.armed_lease().expect(RETIRED_TEST_INSTALL_DETAIL)
}

#[cfg(target_os = "linux")]
fn platform_descriptors() -> CustodyDescriptorSet {
    CustodyDescriptorSet::LinuxContainment
}

#[cfg(target_os = "macos")]
fn platform_descriptors() -> CustodyDescriptorSet {
    CustodyDescriptorSet::macos_process_capability(
        RETIRED_TEST_READER_HANDLE,
        RETIRED_TEST_WRITER_HANDLE,
    )
}

#[cfg(target_os = "linux")]
fn arm_platform(
    pending: CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) -> CustodyDurableRecord {
    pending
        .arm(request, offer)
        .expect(RETIRED_TEST_INSTALL_DETAIL)
}

#[cfg(target_os = "macos")]
fn arm_platform(
    pending: CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) -> CustodyDurableRecord {
    let gone = crate::host::reaped_host_test_process_identity();
    let guardian = crate::pty_custody::CustodyGuardianBinding::new(
        uuid::Uuid::new_v4(),
        crate::pty_custody::CustodyHolderIdentity {
            pid: gone.pid,
            start_time: gone.start_time,
        },
        request,
        offer,
    )
    .expect(RETIRED_TEST_INSTALL_DETAIL);
    pending
        .prepare_guardian(request, offer, guardian)
        .and_then(|prepared| prepared.arm_prepared_guardian(request, offer, guardian))
        .expect(RETIRED_TEST_INSTALL_DETAIL)
}
