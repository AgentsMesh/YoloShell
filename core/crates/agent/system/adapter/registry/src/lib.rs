//! Provider-neutral adapter registry. Provider adapters and their environment
//! keys are injected by the process composition root; this crate never names a
//! provider and never constructs the compiled-in runtime.
use anyhow::Result;
use std::collections::BTreeSet;
use std::sync::Arc;
use yolo_agent_adapter_contract::{
    AgentAdapter, DetectionContext, DetectionEnvironment, LaunchEnvironment,
    constants as adapter_constants,
};
use yolo_agent_contract::{AgentBinding, AgentKind};

#[derive(Clone)]
pub struct AgentRegistry {
    adapters: Vec<Arc<dyn AgentAdapter>>,
    environment_keys: BTreeSet<&'static str>,
}

impl std::fmt::Debug for AgentRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentRegistry")
            .field("adapters", &self.adapters.len())
            .field("environment_keys", &self.environment_keys.len())
            .finish()
    }
}

impl AgentRegistry {
    pub fn from_adapters(adapters: Vec<Box<dyn AgentAdapter>>) -> Result<Self> {
        Self::from_shared_adapters(adapters.into_iter().map(Arc::from).collect())
    }

    pub fn from_shared_adapters(adapters: Vec<Arc<dyn AgentAdapter>>) -> Result<Self> {
        Self::from_shared_adapters_and_keys(adapters, std::iter::empty())
    }

    pub fn from_shared_adapters_and_keys(
        adapters: Vec<Arc<dyn AgentAdapter>>,
        keys: impl IntoIterator<Item = &'static str>,
    ) -> Result<Self> {
        let mut kinds = BTreeSet::new();
        for adapter in &adapters {
            let kind = adapter.kind();
            anyhow::ensure!(
                !kind.as_str().trim().is_empty(),
                yolo_agent_adapter_contract::constants::message::REGISTRY_KIND_EMPTY
            );
            anyhow::ensure!(
                kinds.insert(kind),
                yolo_agent_adapter_contract::constants::message::REGISTRY_KIND_DUPLICATE
            );
        }
        Ok(Self {
            adapters,
            environment_keys: keys.into_iter().collect(),
        })
    }

    pub fn adapters(&self) -> impl Iterator<Item = &dyn AgentAdapter> {
        self.adapters.iter().map(Arc::as_ref)
    }

    pub fn shared_adapters(&self) -> impl Iterator<Item = Arc<dyn AgentAdapter>> + '_ {
        self.adapters.iter().cloned()
    }

    pub fn adapter(&self, kind: &AgentKind) -> Option<&dyn AgentAdapter> {
        self.adapters().find(|adapter| adapter.kind() == *kind)
    }

    pub fn process_environment_keys(&self) -> Vec<&'static str> {
        let mut keys = BTreeSet::from([
            adapter_constants::HOME_ENV,
            adapter_constants::YOLO_TERMINAL_ID_ENV,
            adapter_constants::YOLO_RUNTIME_EPOCH_ENV,
            yolo_process::PTY_SESSION_CAPABILITY_ENV,
        ]);
        keys.extend(self.environment_keys.iter().copied());
        keys.extend(
            self.adapters()
                .filter_map(AgentAdapter::data_home_environment_variable),
        );
        keys.into_iter().collect()
    }

    pub fn detection_context(
        &self,
        terminal_id: uuid::Uuid,
        expected_cwd: std::path::PathBuf,
        overrides: &LaunchEnvironment,
    ) -> Option<DetectionContext> {
        self.detection_context_with_environment(
            terminal_id,
            expected_cwd,
            &DetectionEnvironment::current(overrides),
        )
    }

    pub fn detection_context_with_environment(
        &self,
        terminal_id: uuid::Uuid,
        expected_cwd: std::path::PathBuf,
        environment: &DetectionEnvironment,
    ) -> Option<DetectionContext> {
        let homes: std::collections::BTreeMap<_, _> = self
            .adapters()
            .filter_map(|adapter| {
                adapter
                    .resolve_data_home(environment)
                    .map(|home| (adapter.kind(), home))
            })
            .collect();
        (!homes.is_empty()).then(|| DetectionContext::new(terminal_id, expected_cwd, homes))
    }

    pub fn validate_binding(
        &self,
        binding: &AgentBinding,
        context: &DetectionContext,
    ) -> Result<()> {
        let adapter = self.validate_binding_structure(binding, context)?;
        yolo_agent_adapter_contract::validation::validate_evidence_binding(
            adapter, binding, context,
        )
    }

    /// Validates a binding that a native authority record already authorized.
    /// The record verified the evidence identity it captured; the adapter
    /// evidence inspection is skipped because a provider may legitimately have
    /// no evidence file yet. Some providers create their transcript only on
    /// the first prompt, after the session-start hook has authorized the agent.
    pub fn validate_authority_binding(
        &self,
        binding: &AgentBinding,
        context: &DetectionContext,
    ) -> Result<()> {
        self.validate_binding_structure(binding, context)
            .map(|_| ())
    }

    fn validate_binding_structure<'a>(
        &'a self,
        binding: &AgentBinding,
        context: &DetectionContext,
    ) -> Result<&'a dyn AgentAdapter> {
        yolo_agent_adapter_contract::validation::validate_core(binding, context)?;
        let adapter = self.adapter(&binding.kind).ok_or_else(|| {
            anyhow::anyhow!(yolo_agent_adapter_contract::constants::message::KIND)
        })?;
        yolo_agent_adapter_contract::validation::validate_recovery_availability(adapter, binding)?;
        yolo_agent_adapter_contract::validation::validate_evidence_ownership(
            adapter, binding, context,
        )?;
        adapter.validate_binding(binding, context)?;
        Ok(adapter)
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
