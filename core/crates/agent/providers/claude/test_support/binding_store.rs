use crate::agent::claude::generation::{
    AuthorityRuntimeIdentity, ClaudeGenerationAuthority, GenerationBegin,
};
use crate::agent::{AgentBinding, DetectionContext, ObservedProcess};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(crate) struct AgentBindingStore {
    inner: yolo_agent_store::AgentBindingStore,
}

impl AgentBindingStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: yolo_agent_store::AgentBindingStore::new(path),
        }
    }

    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    pub fn clear(&self) -> Result<()> {
        self.inner.clear()
    }

    pub fn read_raw_in_namespace(&self, lease: &NamespaceLease) -> Result<Option<AgentBinding>> {
        yolo_agent_binding_io::AgentBindingFile::new(self.path()).read_in_namespace(lease)
    }

    pub fn save_authoritative_in_namespace(
        &self,
        lease: &NamespaceLease,
        binding: &AgentBinding,
        authority: &ClaudeGenerationAuthority,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<()> {
        anyhow::ensure!(
            authority.matches_runtime(runtime) && authority.matches_binding_payload(binding),
            "Claude test binding does not match authority"
        );
        lease.write_json(self.path(), binding)
    }

    pub fn invalidate_in_namespace(
        &self,
        lease: &NamespaceLease,
        session_id: uuid::Uuid,
    ) -> Result<Option<AgentBinding>> {
        self.inner.invalidate_in_namespace(lease, session_id)
    }

    pub fn load_authoritative_in_namespace(
        &self,
        lease: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<Option<AgentBinding>> {
        self.inner
            .load_authoritative_in_namespace(lease, context, runtime)
    }

    pub fn load_published_authoritative_in_namespace(
        &self,
        lease: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<Option<AgentBinding>> {
        self.inner
            .load_published_authoritative_in_namespace(lease, context, runtime)
    }

    pub fn load_published_authoritative_with_test_hook(
        &self,
        lease: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        hook: impl FnOnce() -> Result<()>,
    ) -> Result<Option<AgentBinding>> {
        self.inner.load_published_authoritative_with_test_hook(
            lease, context, runtime, hook,
        )
    }

    pub fn reconcile_detected_in_namespace(
        &self,
        lease: &NamespaceLease,
        binding: &AgentBinding,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<AgentBinding> {
        self.inner
            .reconcile_detected_with_commit_fence(lease, binding, runtime, || Ok(()))
    }

    pub fn reactivate_claude_foreground_in_namespace<F>(
        &self,
        lease: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        processes: &[ObservedProcess],
        observe: F,
    ) -> Result<Option<AgentBinding>>
    where
        F: FnMut() -> Result<Vec<ObservedProcess>>,
    {
        self.inner.reactivate_claude_foreground_in_namespace(
            lease, context, runtime, processes, observe,
        )
    }

    pub fn begin_managed_claude_in_namespace(
        &self,
        lease: &NamespaceLease,
        begin: &GenerationBegin,
        foreground: bool,
    ) -> Result<ClaudeGenerationAuthority> {
        let encoded = serde_json::to_value(begin)?;
        let begin = serde_json::from_value(encoded)?;
        let authority = self
            .inner
            .begin_managed_claude_in_namespace(lease, &begin, foreground)?;
        serde_json::from_value(serde_json::to_value(authority)?)
            .context("convert managed Claude test authority")
    }
}
