use super::{CapabilityScope, PtySessionProof};

impl PtySessionProof {
    pub(super) fn is_custody_pty_object(&self) -> bool {
        self.scope == CapabilityScope::CustodyPtyObject
    }
}
