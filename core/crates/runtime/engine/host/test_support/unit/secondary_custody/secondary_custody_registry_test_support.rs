use super::*;

#[path = "secondary_custody_registry_retired_test_support.rs"]
mod retired;

const RETIRED_TEST_DIGEST_BYTE: u8 = 0x63;
const RETIRED_TEST_STREAM_DETAIL: &str = "create retired custody test stream pair";
const RETIRED_TEST_INSTALL_DETAIL: &str = "install retired custody test lease";
const RETIRED_TEST_SETTLEMENT_DETAIL: &str = "prove retired custody test settlement";
const RETIRED_TEST_COMMIT_DETAIL: &str = "commit retired custody test lease";
const ACTIVE_TEST_LEASE_MISSING_DETAIL: &str = "active custody test lease is unavailable";
const ACTIVE_TEST_LEASE_MISMATCH_DETAIL: &str = "active custody test lease changed identity";

#[derive(Clone, Copy)]
pub(in crate::host) enum TestRetiredCustodyState {
    ReleaseAckPending,
    Released,
    ReleaseUncertain,
    Superseded,
}

impl SecondaryCustodyRegistry {
    pub(in crate::host) fn complete_test_pending_release(
        &self,
        lease: &Arc<SecondaryCustodyLease>,
    ) {
        lease
            .resolve_pending_release(SecondaryCustodyState::Released)
            .expect(RETIRED_TEST_COMMIT_DETAIL);
    }

    pub(in crate::host) fn install_test_retirement_evidence(
        &self,
        host: &Host,
        record: &RuntimeRecord,
    ) {
        retired::publish_synthetic_receipt(host, record);
    }

    pub(in crate::host) fn retained_retirement_count(&self) -> usize {
        super::super::lock_recover(&self.entries.values)
            .retired
            .values()
            .filter(|retirement| retirement.retains_lease())
            .count()
    }

    pub(in crate::host) fn terminal_retirement_count(&self) -> usize {
        super::super::lock_recover(&self.entries.values)
            .retired
            .values()
            .filter(|retirement| !retirement.retains_lease())
            .count()
    }

    pub(in crate::host) fn install_test_active_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
    ) -> Arc<SecondaryCustodyLease> {
        let lease = self.install_test_lease(host, record);
        lease.set_state(SecondaryCustodyState::Ready);
        lease
    }

    pub(in crate::host) fn disconnect_test_active_lease(&self, lease: &Arc<SecondaryCustodyLease>) {
        self.assert_test_lease_active(lease);
        lease.set_state(SecondaryCustodyState::Disconnected);
    }

    pub(in crate::host) fn retire_test_disconnected_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
        lease: &Arc<SecondaryCustodyLease>,
    ) {
        self.assert_test_lease_active(lease);
        let settlement = super::settlement::prove_exact(lease)
            .expect(RETIRED_TEST_SETTLEMENT_DETAIL)
            .expect(RETIRED_TEST_SETTLEMENT_DETAIL);
        lease
            .commit_settled_release(settlement)
            .expect(RETIRED_TEST_COMMIT_DETAIL);
        retired::publish_synthetic_receipt(host, record);
    }

    fn assert_test_lease_active(&self, lease: &Arc<SecondaryCustodyLease>) {
        let current = self
            .current(&lease.runtime)
            .expect(ACTIVE_TEST_LEASE_MISSING_DETAIL);
        assert!(
            Arc::ptr_eq(&current, lease),
            "{ACTIVE_TEST_LEASE_MISMATCH_DETAIL}"
        );
    }

    fn install_test_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
    ) -> Arc<SecondaryCustodyLease> {
        let request = crate::pty_custody::CustodyRequest::new(
            crate::pty_custody::CustodyLeaseId::new(),
            CustodyRuntimeIdentity::from_record(record),
            host.secondary_custody_holder,
            record.auth_token.clone(),
            record.controller_generation,
        );
        let offer = crate::host::test_custody_offer(
            &request,
            crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor,
            crate::sha256_digest::Sha256Digest::repeated(RETIRED_TEST_DIGEST_BYTE),
        );
        host.ensure_session_namespace(record.terminal_id)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        host.namespace
            .write_json(&host.paths.runtime_record(record.terminal_id), record)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        let pending = crate::pty_custody::CustodyDurableRecord::pending(request.runtime.clone());
        let armed = crate::host::test_arm_custody(&pending, &request, &offer)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        host.namespace
            .write_json(&host.paths.pty_custody(record.terminal_id), &armed)
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        let (stream, _peer) =
            crate::local_unix::LocalUnixStream::pair().expect(RETIRED_TEST_STREAM_DETAIL);
        let stream = CustodyUnixStream::from_local(stream).expect(RETIRED_TEST_STREAM_DETAIL);
        let lease = Arc::new(
            SecondaryCustodyLease::new(
                host,
                &request,
                offer,
                None,
                PlatformSecondaryCustody::ReleasedSuccessor,
                stream,
            )
            .expect(RETIRED_TEST_INSTALL_DETAIL),
        );
        self.install(Arc::clone(&lease))
            .expect(RETIRED_TEST_INSTALL_DETAIL);
        lease
    }
}
