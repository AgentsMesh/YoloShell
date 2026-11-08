use super::*;
use yolo_agent_hook_generation::HookGenerationStore;

impl NativeAuthority {
    pub(in crate::store) fn verify(
        &self,
        store: &AgentBindingStore,
        namespace: &NamespaceLease,
    ) -> Result<()> {
        let (path, identity) = match self {
            Self::Claude(_, identity) => (
                ClaudeNativeGenerationStore::from_binding_path(store.path())?
                    .path()
                    .to_owned(),
                *identity,
            ),
            Self::Codex(_, identity) => {
                (store.codex_authority_store()?.path().to_owned(), *identity)
            }
            Self::GenericHook(_, identity) => (
                HookGenerationStore::from_binding_path(store.path())?
                    .path()
                    .to_owned(),
                *identity,
            ),
        };
        namespace.verify()?;
        namespace
            .file(&path)?
            .verify_exact_identity(identity, crate::paths::MAX_DURABLE_JSON_BYTES)?;
        namespace.verify()
    }

    pub(in crate::store) fn matches_subject(&self, binding: &AgentBinding) -> bool {
        let canonical = self.original();
        self.matches_observation(binding)
            && canonical.executable == binding.executable
            && canonical.detected_at_ms == binding.detected_at_ms
    }

    pub(in crate::store) fn matches_observation(&self, binding: &AgentBinding) -> bool {
        let mut canonical = self.original().clone();
        if binding.state == AgentState::Exited
            && match self {
                Self::Claude(record, _) => record.final_session_end,
                Self::GenericHook(record, _) => record.final_session_end,
                Self::Codex(_, _) => false,
            }
        {
            canonical.process_pid = None;
            canonical.process_start_time = None;
        }
        super::super::observation::same(&canonical, binding)
    }
}
