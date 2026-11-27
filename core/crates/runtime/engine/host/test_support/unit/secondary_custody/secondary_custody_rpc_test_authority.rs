use super::*;

impl SecondaryCustodyRegistry {
    pub(in crate::host) fn authorize_rpc_test(&self, record: &RuntimeRecord) {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        let mut authorities = super::super::lock_recover(&self.rpc_test_authority);
        if !authorities.contains(&runtime) {
            authorities.push(runtime);
        }
    }

    pub(in crate::host) fn rpc_test_authorized(&self, record: &RuntimeRecord) -> bool {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        super::super::lock_recover(&self.rpc_test_authority).contains(&runtime)
    }

    pub(in crate::host) fn revoke_rpc_test(&self, record: &RuntimeRecord) {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        super::super::lock_recover(&self.rpc_test_authority)
            .retain(|authorized| authorized != &runtime);
    }

    pub(in crate::host) fn authorize_peer_for_test(
        &self,
        record: &RuntimeRecord,
        peer: crate::process::ProcessIdentity,
    ) {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        let mut authorities = super::super::lock_recover(&self.peer_test_authority);
        authorities.retain(|(authorized, _)| authorized != &runtime);
        authorities.push((runtime, peer));
    }

    pub(super) fn peer_test_authorized(
        &self,
        record: &RuntimeRecord,
        peer: crate::process::ProcessIdentity,
    ) -> bool {
        self.authorized_peer_for_test(record) == Some(peer)
    }

    pub(in crate::host) fn authorized_peer_for_test(
        &self,
        record: &RuntimeRecord,
    ) -> Option<crate::process::ProcessIdentity> {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        super::super::lock_recover(&self.peer_test_authority)
            .iter()
            .find_map(|(authorized, peer)| (*authorized == runtime).then_some(*peer))
    }
}
