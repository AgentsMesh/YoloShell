use super::{AuthorityAuthentication, AuthorityRuntimeIdentity, ClaudeGenerationAuthority};
use crate::agent::AgentBinding;

impl ClaudeGenerationAuthority {
    pub fn authentication(&self) -> AuthorityAuthentication {
        AuthorityAuthentication {
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            invocation_id: self.invocation_id,
            hook_token: self.hook_token,
            helper_capability: self.helper_capability,
            process: self.process,
        }
    }

    pub fn binding(&self, previous: Option<&AgentBinding>) -> Option<AgentBinding> {
        super::binding::from_authority(self, previous)
    }

    pub fn transition_binding(&self, previous: Option<&AgentBinding>) -> Option<AgentBinding> {
        super::binding::transition_target(self, previous)
    }

    pub fn matches_unknown_binding(&self, binding: &AgentBinding) -> bool {
        super::binding::unknown_matches(self, binding)
    }

    pub fn binding_subject_matches(&self, binding: &AgentBinding) -> bool {
        super::binding::subject_matches(self, binding)
    }

    pub fn matches_binding(&self, binding: &AgentBinding) -> bool {
        super::record::matches_binding(self, binding)
    }

    pub fn matches_binding_payload(&self, binding: &AgentBinding) -> bool {
        super::record::matches_binding_payload(self, binding)
    }

    pub fn matches_runtime(&self, runtime: AuthorityRuntimeIdentity) -> bool {
        self.runtime_epoch == runtime.runtime_epoch && self.worker == runtime.worker
    }

    pub fn transcript_identity_is_current(&self) -> bool {
        match (&self.transcript_path, self.transcript_identity) {
            (None, None) => true,
            (Some(path), Some(expected)) => {
                super::super::authoritative_evidence::named_transcript_identity(
                    &self.data_home,
                    path,
                )
                .ok()
                .flatten()
                    == Some(expected)
            }
            _ => false,
        }
    }
}
