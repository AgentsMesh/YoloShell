use crate::hook_client::HookClient;
use crate::{ProviderRuntimeError, TurnEndProbe};
use std::sync::Arc;
use yolo_agent_adapter_contract::AgentAdapter;
use yolo_agent_contract::AgentKind;
use yolo_agent_provider_contract::{
    InteractionSemantics, InterruptDelivery, PromptSubmit, ProviderDescriptor,
};
use yolo_conversation_source::ConversationSource;
use yolo_hook_codec::HookCodec;
use yolo_hook_installation_contract::HookInstallSpec;

/// One compiled-in provider: metadata plus the implemented ports. Additional
/// optional ports are added as their owning contracts land.
#[derive(Clone)]
pub struct ProviderRuntime {
    descriptor: ProviderDescriptor,
    adapter: Arc<dyn AgentAdapter>,
    process_environment_keys: &'static [&'static str],
    turn_end_probe: Option<Arc<dyn TurnEndProbe>>,
    prompt_submit: PromptSubmit,
    interrupt_delivery: InterruptDelivery,
    hook_client: HookClient,
    hook_codec: Option<Arc<dyn HookCodec>>,
    install_spec: Option<&'static HookInstallSpec>,
    conversation_source: Option<Arc<dyn ConversationSource>>,
    interaction_semantics: InteractionSemantics,
    launch_command: Option<&'static str>,
}

impl ProviderRuntime {
    pub fn new(
        descriptor: ProviderDescriptor,
        adapter: Arc<dyn AgentAdapter>,
    ) -> Result<Self, ProviderRuntimeError> {
        if adapter.kind() != *descriptor.kind() {
            return Err(ProviderRuntimeError::AdapterKindMismatch);
        }
        Ok(Self {
            descriptor,
            adapter,
            process_environment_keys: &[],
            turn_end_probe: None,
            prompt_submit: PromptSubmit::Atomic,
            interrupt_delivery: InterruptDelivery::Unsupported,
            hook_client: HookClient::Ingress,
            hook_codec: None,
            install_spec: None,
            conversation_source: None,
            interaction_semantics: InteractionSemantics::None,
            launch_command: None,
        })
    }

    pub fn with_process_environment_keys(mut self, keys: &'static [&'static str]) -> Self {
        self.process_environment_keys = keys;
        self
    }

    pub fn with_turn_end_probe(mut self, probe: Arc<dyn TurnEndProbe>) -> Self {
        self.turn_end_probe = Some(probe);
        self
    }

    pub fn with_prompt_submit(mut self, prompt_submit: PromptSubmit) -> Self {
        self.prompt_submit = prompt_submit;
        self
    }

    pub fn prompt_submit(&self) -> PromptSubmit {
        self.prompt_submit
    }

    pub fn with_interrupt_delivery(mut self, interrupt_delivery: InterruptDelivery) -> Self {
        self.interrupt_delivery = interrupt_delivery;
        self
    }

    pub fn interrupt_delivery(&self) -> InterruptDelivery {
        self.interrupt_delivery
    }

    pub fn process_environment_keys(&self) -> &'static [&'static str] {
        self.process_environment_keys
    }

    pub fn turn_end_probe(&self) -> Option<&Arc<dyn TurnEndProbe>> {
        self.turn_end_probe.as_ref()
    }

    pub fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    pub fn adapter(&self) -> &Arc<dyn AgentAdapter> {
        &self.adapter
    }

    pub fn kind(&self) -> AgentKind {
        self.descriptor.kind().clone()
    }

    pub fn with_hook_client(mut self, hook_client: HookClient) -> Self {
        self.hook_client = hook_client;
        self
    }

    pub fn hook_client(&self) -> HookClient {
        self.hook_client
    }

    pub fn with_hook_codec(mut self, codec: Arc<dyn HookCodec>) -> Result<Self, ProviderRuntimeError> {
        if codec.kind() != self.kind() || codec.profile().provider != self.descriptor.install_token() {
            return Err(ProviderRuntimeError::HookCodecMismatch);
        }
        self.hook_codec = Some(codec);
        Ok(self)
    }

    pub fn hook_codec(&self) -> Option<&Arc<dyn HookCodec>> {
        self.hook_codec.as_ref()
    }

    pub fn with_install_spec(mut self, spec: &'static HookInstallSpec) -> Result<Self, ProviderRuntimeError> {
        if spec.provider != self.descriptor.install_token() {
            return Err(ProviderRuntimeError::InstallSpecMismatch);
        }
        self.install_spec = Some(spec);
        Ok(self)
    }

    pub fn install_spec(&self) -> Option<&'static HookInstallSpec> {
        self.install_spec
    }

    pub fn with_conversation_source(
        mut self,
        source: Arc<dyn ConversationSource>,
    ) -> Result<Self, ProviderRuntimeError> {
        if source.kind() != self.kind()
            || source.conversation_token().as_str() != self.descriptor.conversation_token()
        {
            return Err(ProviderRuntimeError::ConversationSourceMismatch);
        }
        self.conversation_source = Some(source);
        Ok(self)
    }

    pub fn conversation_source(&self) -> Option<&Arc<dyn ConversationSource>> {
        self.conversation_source.as_ref()
    }

    pub fn with_interaction_semantics(mut self, semantics: InteractionSemantics) -> Self {
        self.interaction_semantics = semantics;
        self
    }

    pub fn interaction_semantics(&self) -> InteractionSemantics {
        self.interaction_semantics
    }

    pub fn with_launch_command(mut self, command: &'static str) -> Self {
        self.launch_command = Some(command);
        self
    }

    pub fn launch_command(&self) -> Option<&'static str> {
        self.launch_command
    }
}
