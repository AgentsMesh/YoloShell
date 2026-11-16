use super::PtyOwnership;
use super::custody::ArmedPtyOwnership;
use crate::worker::pty::pty_fault_reaper::OwnershipFaultHandoff;

#[cfg(test)]
pub(crate) const INJECTED_EXTRACTION_PANIC: &str =
    "injected PTY ownership handoff extraction panic";
pub(crate) const CUSTODY_MISSING_DETAIL: &str = "extracted PTY ownership custody is present";
pub(crate) const COMPLETION_MISSING_DETAIL: &str = "extracted PTY completion publisher is present";
pub(crate) const HANDOFF_MISSING_DETAIL: &str = "assembled PTY ownership handoff is present";
pub(crate) const OWNERSHIP_MISSING_DETAIL: &str = "armed PTY ownership is present";

pub(crate) struct OwnershipHandoffExtraction<'owner> {
    pub(crate) owner: &'owner mut PtyOwnership,
    pub(crate) armed: Option<ArmedPtyOwnership>,
    pub(crate) handoff: Option<OwnershipFaultHandoff>,
    pub(crate) transferred: bool,
}

impl<'owner> OwnershipHandoffExtraction<'owner> {
    pub(crate) fn capture(owner: &'owner mut PtyOwnership) -> Self {
        let mut extraction = Self {
            owner,
            armed: None,
            handoff: None,
            transferred: false,
        };
        extraction.armed = extraction.owner.armed_ownership.take();
        extraction.raise_for_test(HandoffExtractionPhase::CustodyCaptured);
        extraction
    }

    pub(crate) fn install(mut self) {
        self.assert_captured();
        let armed = self.armed.as_mut().expect(OWNERSHIP_MISSING_DETAIL);
        let custody = armed.custody.take().expect(CUSTODY_MISSING_DETAIL);
        let completion = armed
            .fault_completion
            .take()
            .expect(COMPLETION_MISSING_DETAIL);
        self.handoff = Some(OwnershipFaultHandoff::with_failure(
            custody,
            completion,
            armed.fault.take(),
        ));
        #[cfg(test)]
        let observation = self
            .handoff
            .as_ref()
            .and_then(|handoff| handoff.custody().processes.as_ref())
            .map(crate::worker::pty_process_custody::PtyProcessCustody::snapshot_for_test);
        self.raise_for_test(HandoffExtractionPhase::HandoffAssembled);
        let transaction = self
            .armed
            .as_mut()
            .expect(OWNERSHIP_MISSING_DETAIL)
            .fault_reaper
            .prepare_ownership_handoff();
        transaction.install(&mut self.handoff);
        assert!(self.handoff.is_none(), "{HANDOFF_MISSING_DETAIL}");
        self.transferred = true;
        #[cfg(test)]
        if let Some(observation) = observation {
            self.armed
                .as_ref()
                .expect(OWNERSHIP_MISSING_DETAIL)
                .fault_reaper
                .observe_handoff_snapshot(observation);
        }
    }

    pub(crate) fn assert_captured(&self) {
        assert!(
            self.armed
                .as_ref()
                .is_some_and(|armed| armed.custody.is_some()),
            "{CUSTODY_MISSING_DETAIL}"
        );
        assert!(
            self.armed
                .as_ref()
                .is_some_and(|armed| armed.fault_completion.is_some()),
            "{COMPLETION_MISSING_DETAIL}"
        );
    }

    #[cfg(test)]
    pub(crate) fn raise_for_test(&mut self, phase: HandoffExtractionPhase) {
        if self.owner.handoff_extraction_panic == Some(phase) {
            self.owner.handoff_extraction_panic = None;
            panic!("{INJECTED_EXTRACTION_PANIC}");
        }
    }

    #[cfg(not(test))]
    pub(crate) fn raise_for_test(&mut self, _phase: HandoffExtractionPhase) {}
}

impl Drop for OwnershipHandoffExtraction<'_> {
    fn drop(&mut self) {
        if self.transferred {
            return;
        }
        let Some(mut armed) = self.armed.take() else {
            return;
        };
        if let Some(handoff) = self.handoff.take() {
            armed.custody = Some(handoff.custody);
            armed.fault_completion = Some(handoff.completion);
            armed.fault = handoff.failure;
        }
        self.owner.armed_ownership = Some(armed);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HandoffExtractionPhase {
    CustodyCaptured,
    HandoffAssembled,
}

#[cfg(test)]
impl HandoffExtractionPhase {
    pub(crate) const ALL: &'static [Self] = &[Self::CustodyCaptured, Self::HandoffAssembled];
}

impl PtyOwnership {
    #[cfg(test)]
    pub(crate) fn panic_after_handoff_extraction_for_test(
        &mut self,
        phase: HandoffExtractionPhase,
    ) {
        assert!(self.handoff_extraction_panic.replace(phase).is_none());
    }
}
