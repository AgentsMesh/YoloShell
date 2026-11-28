use super::super::{FrozenProcessTree, ProcessClosureVersion};

impl FrozenProcessTree {
    pub fn clear_closure_for_test(&mut self) {
        self.closure.clear();
    }

    pub fn deny_destructive_for_test(&mut self) {
        self.tree.deny_destructive_for_test();
    }
}

pub fn assert_gone_proof_version(current: ProcessClosureVersion, proven: ProcessClosureVersion) {
    assert_eq!(
        current,
        proven,
        "{}",
        super::GONE_PROOF_VERSION_MISMATCH_DETAIL
    );
}
