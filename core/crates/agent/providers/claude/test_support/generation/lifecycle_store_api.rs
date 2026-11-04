use super::*;

impl ClaudeGenerationStore {
    pub fn transition_foreground_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        sequence: u64,
        event: &GenerationEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition, bool)> {
        self.transition_in_namespace(
            namespace,
            authentication,
            expected,
            sequence,
            event,
            transcript_identity,
            false,
        )
    }
}
