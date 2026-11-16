use std::sync::atomic::{AtomicU8, Ordering};

/// A live condition that prevents the Worker from proving durable output.
///
/// `OutputBlocked` is a current status, not an exit latch. Each cause is
/// cleared by the path that resolves it. Terminal causes remain set until the
/// durable exit record supersedes the runtime status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum OutputBlockCause {
    FinalDrainDeadline,
    ReaderRestartsExhausted,
    ExitPersistence,
    DeferredCleanup,
    LostOutputAbandonment,
    NamespaceOwnershipLoss,
}

#[cfg(test)]
const ALL_CAUSES: [OutputBlockCause; 6] = [
    OutputBlockCause::FinalDrainDeadline,
    OutputBlockCause::ReaderRestartsExhausted,
    OutputBlockCause::ExitPersistence,
    OutputBlockCause::DeferredCleanup,
    OutputBlockCause::LostOutputAbandonment,
    OutputBlockCause::NamespaceOwnershipLoss,
];

impl OutputBlockCause {
    const fn mask(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Debug, Default)]
pub(crate) struct OutputBlock {
    causes: AtomicU8,
}

impl OutputBlock {
    /// Returns true when this cause was not already recorded.
    pub(crate) fn mark(&self, cause: OutputBlockCause) -> bool {
        self.causes.fetch_or(cause.mask(), Ordering::AcqRel) & cause.mask() == 0
    }

    pub(crate) fn clear(&self, cause: OutputBlockCause) {
        self.causes.fetch_and(!cause.mask(), Ordering::AcqRel);
    }

    pub(crate) fn blocked(&self) -> bool {
        self.causes.load(Ordering::Acquire) != 0
    }
}

#[cfg(test)]
impl OutputBlock {
    pub(crate) fn causes_for_test(&self) -> impl Iterator<Item = OutputBlockCause> {
        let causes = self.causes.load(Ordering::Acquire);
        ALL_CAUSES
            .into_iter()
            .filter(move |cause| causes & cause.mask() != 0)
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/output/output_block_tests.rs"]
mod tests;
