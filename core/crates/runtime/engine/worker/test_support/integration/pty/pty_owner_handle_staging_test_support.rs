use super::*;

impl StagedPtyOwner {
    pub(in crate::worker) fn inject_construction_panic(&mut self, detail: &'static str) {
        self.construction_panic = Some(detail);
    }

    pub(in crate::worker) fn block_publication_after_custody_drop(
        &mut self,
        barrier: Arc<BlockingControlHook>,
    ) {
        self.publication_barrier = Some(barrier);
    }

    pub(in crate::worker) fn settlement_for_test(&self) -> OwnerSettlement {
        Arc::clone(&self.handle.settlement)
    }
}
