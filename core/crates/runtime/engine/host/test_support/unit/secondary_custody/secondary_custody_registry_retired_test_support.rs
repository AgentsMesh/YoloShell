use super::*;

#[path = "secondary_custody_registry_retired_receipt_test_support.rs"]
mod receipt;

impl SecondaryCustodyRegistry {
    pub(in crate::host) fn install_test_retired_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
        state: TestRetiredCustodyState,
    ) -> Arc<SecondaryCustodyLease> {
        if matches!(state, TestRetiredCustodyState::Released) {
            return self.install_test_released_lease(host, record);
        }
        let lease = self.install_test_lease(host, record);
        match state {
            TestRetiredCustodyState::ReleaseAckPending => {
                lease.set_state(SecondaryCustodyState::ReleaseAckPending);
                self.commit_test_retained(&lease);
            }
            TestRetiredCustodyState::ReleaseUncertain => {
                lease.set_state(SecondaryCustodyState::ReleaseUncertain);
                self.commit_test_retained(&lease);
            }
            TestRetiredCustodyState::Superseded => {
                lease.set_state(SecondaryCustodyState::Ready);
                lease.commit_superseded().expect(RETIRED_TEST_COMMIT_DETAIL);
            }
            TestRetiredCustodyState::Released => unreachable!(),
        }
        lease
    }

    fn install_test_released_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
    ) -> Arc<SecondaryCustodyLease> {
        let (lease, prepared) = self.install_test_receipted_lease(host, record);
        lease.set_state(SecondaryCustodyState::Disconnected);
        let settlement = super::super::settlement::prove_exact(&lease)
            .expect(RETIRED_TEST_SETTLEMENT_DETAIL)
            .expect(RETIRED_TEST_SETTLEMENT_DETAIL);
        lease
            .commit_settled_release(settlement)
            .expect(RETIRED_TEST_COMMIT_DETAIL);
        prepared.publish(host, record);
        lease
    }

    fn commit_test_retained(&self, lease: &Arc<SecondaryCustodyLease>) {
        let mut values = super::super::super::lock_recover(&self.entries.values);
        let retirement = super::super::registry::prepare_exact_retirement(&mut values, lease)
            .expect(RETIRED_TEST_COMMIT_DETAIL);
        retirement.commit_retained(&mut values, lease);
    }

    fn install_test_receipted_lease(
        &self,
        host: &Host,
        record: &RuntimeRecord,
    ) -> (Arc<SecondaryCustodyLease>, receipt::PreparedRetirement) {
        let prepared = receipt::PreparedRetirement::prepare(host, record);
        let request = prepared.request().clone();
        let offer = prepared.offer().clone();
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
        (lease, prepared)
    }
}

pub(super) fn publish_synthetic_receipt(host: &Host, record: &RuntimeRecord) {
    receipt::PreparedRetirement::prepare(host, record).publish(host, record);
}
