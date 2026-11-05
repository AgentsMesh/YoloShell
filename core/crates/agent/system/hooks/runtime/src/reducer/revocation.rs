use crate::reducer::Reducer;
use crate::revocation::RevocationEvidence;
use crate::revocation::predecessor::PreRevocationProof;
use crate::{HookEnvelope, HookViolation, ProcessIdentity};

impl Reducer {
    pub(crate) fn invalidate_event<T>(
        &mut self,
        event: &HookEnvelope,
        violation: HookViolation,
    ) -> Result<T, HookViolation> {
        let predecessor = PreRevocationProof::capture(self)?;
        let evidence = RevocationEvidence::hook(predecessor, event.clone(), violation)?;
        self.invalidate(evidence, violation)
    }

    pub(crate) fn invalidate_process<T>(
        &mut self,
        observed: ProcessIdentity,
        violation: HookViolation,
    ) -> Result<T, HookViolation> {
        let predecessor = PreRevocationProof::capture(self)?;
        self.invalidate(
            RevocationEvidence::process(predecessor, observed, violation)?,
            violation,
        )
    }

    pub fn revocation_evidence(&self) -> Option<&RevocationEvidence> {
        self.state.revocation.as_ref()
    }

    fn invalidate<T>(
        &mut self,
        evidence: RevocationEvidence,
        violation: HookViolation,
    ) -> Result<T, HookViolation> {
        self.revision.advance()?;
        crate::reducer::canonical::canonicalize_revoked(&mut self.state, evidence);
        Err(violation)
    }
}
