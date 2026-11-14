use crate::turn_end::TurnEndProbe;
use crate::{HookClient, ProviderRuntime, ProviderRuntimeError};
use std::collections::BTreeSet;
use std::sync::Arc;
use yolo_agent_adapter_contract::AgentAdapter;
use yolo_agent_contract::AgentKind;
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, ProjectionPolicies, ProviderDescriptor,
};
use yolo_conversation_source::ConversationSource;
use yolo_hook_codec::HookCodec;
use yolo_hook_installation_contract::HookInstallSpec;

#[derive(Clone, Default)]
pub struct ProviderRuntimeRegistry {
    runtimes: Vec<ProviderRuntime>,
}

impl std::fmt::Debug for ProviderRuntimeRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRuntimeRegistry")
            .field("runtimes", &self.runtimes.len())
            .finish()
    }
}

impl ProviderRuntimeRegistry {
    pub fn from_runtimes(
        runtimes: impl IntoIterator<Item = ProviderRuntime>,
    ) -> Result<Self, ProviderRuntimeError> {
        let runtimes: Vec<_> = runtimes.into_iter().collect();
        let mut manifest_ids = BTreeSet::new();
        let mut kinds = BTreeSet::new();
        let mut install_tokens = BTreeSet::new();
        let mut conversation_tokens = BTreeSet::new();
        for runtime in &runtimes {
            let descriptor = runtime.descriptor();
            if !manifest_ids.insert(descriptor.manifest_id().to_owned())
                || !kinds.insert(descriptor.kind().clone())
                || !install_tokens.insert(descriptor.install_token().to_owned())
                || !conversation_tokens.insert(descriptor.conversation_token().to_owned())
            {
                return Err(ProviderRuntimeError::DuplicateIdentity);
            }
            // Typed interactions require a native decision bridge. Claude is
            // the only authority that provides one today; add the new authority
            // here and in the Worker dispatch together.
            if runtime.interaction_semantics() == InteractionSemantics::Typed
                && descriptor.authority_kind() != AuthorityKind::ClaudeGeneration
            {
                return Err(ProviderRuntimeError::InteractionSemanticsMismatch);
            }
        }
        Ok(Self { runtimes })
    }

    pub fn len(&self) -> usize {
        self.runtimes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.runtimes.is_empty()
    }

    pub fn runtimes(&self) -> impl Iterator<Item = &ProviderRuntime> {
        self.runtimes.iter()
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ProviderDescriptor> {
        self.runtimes
            .iter()
            .map(ProviderRuntime::descriptor)
    }

    pub fn get(&self, kind: &AgentKind) -> Option<&ProviderRuntime> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.descriptor().kind() == kind)
    }

    pub fn by_manifest_id(&self, manifest_id: &str) -> Option<&ProviderRuntime> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.descriptor().manifest_id() == manifest_id)
    }

    pub fn by_install_token(&self, token: &str) -> Option<&ProviderRuntime> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.descriptor().install_token() == token)
    }

    pub fn by_conversation_token(&self, token: &str) -> Option<&ProviderRuntime> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.descriptor().conversation_token() == token)
    }

    pub fn adapter(&self, kind: &AgentKind) -> Option<&dyn AgentAdapter> {
        self.get(kind).map(|runtime| runtime.adapter().as_ref())
    }

    pub fn adapters(&self) -> impl Iterator<Item = &dyn AgentAdapter> {
        self.runtimes
            .iter()
            .map(|runtime| runtime.adapter().as_ref())
    }

    pub fn turn_end_probe(&self, kind: &AgentKind) -> Option<&Arc<dyn TurnEndProbe>> {
        self.get(kind)?.turn_end_probe()
    }

    pub fn process_environment_keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.runtimes
            .iter()
            .flat_map(|runtime| runtime.process_environment_keys().iter().copied())
    }

    pub fn projection_policies(&self) -> ProjectionPolicies {
        ProjectionPolicies::from_descriptors(self.descriptors())
    }

    pub fn shared_adapters(&self) -> impl Iterator<Item = std::sync::Arc<dyn AgentAdapter>> + '_ {
        self.runtimes
            .iter()
            .map(|runtime| runtime.adapter().clone())
    }

    /// Generic ingress codecs, one per provider that uses the shared engine.
    pub fn hook_codecs(&self) -> Vec<Arc<dyn HookCodec>> {
        self.runtimes
            .iter()
            .filter_map(|runtime| runtime.hook_codec().cloned())
            .collect()
    }

    pub fn hook_codec(&self, install_token: &str) -> Option<Arc<dyn HookCodec>> {
        self.by_install_token(install_token)?
            .hook_codec()
            .cloned()
    }

    pub fn install_specs(&self) -> Vec<&'static HookInstallSpec> {
        self.runtimes
            .iter()
            .filter_map(ProviderRuntime::install_spec)
            .collect()
    }

    pub fn install_spec(&self, install_token: &str) -> Option<&'static HookInstallSpec> {
        self.by_install_token(install_token)?.install_spec()
    }

    pub fn conversation_sources(&self) -> Vec<Arc<dyn ConversationSource>> {
        self.runtimes
            .iter()
            .filter_map(|runtime| runtime.conversation_source().cloned())
            .collect()
    }

    pub fn hook_client(&self, install_token: &str) -> Option<HookClient> {
        self.by_install_token(install_token).map(ProviderRuntime::hook_client)
    }
}
