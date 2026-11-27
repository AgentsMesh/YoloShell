use super::*;

impl RuntimeCleanup {
    pub(crate) fn bound_records_for_test(&self) -> Vec<RuntimeRecord> {
        self.records
            .iter()
            .map(|record| record.record().clone())
            .collect()
    }

    pub(crate) fn pending_records_for_test(&self) -> Vec<RuntimeRecord> {
        self.unbound.clone()
    }

    pub(crate) fn tree_identities_for_test(&self) -> Vec<RuntimeProcessIdentity> {
        self.trees.identities()
    }
}
