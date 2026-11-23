//! Grok Build's compiled-in runtime row. The mechanism validates every
//! identity below; a mismatch fails at startup.
use crate::conversation::GrokConversationSource;
use crate::install::install_spec;
use crate::{CONVERSATION_TOKEN, DISPLAY_NAME, identity};
use std::sync::Arc;
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, InterruptDelivery, LifecycleSemantics, ProjectionPolicy, PromptSubmit,
    ProviderCapabilities, ProviderDescriptor, ProviderTier,
};
use yolo_agent_provider_runtime::{HookClient, ProviderRuntime};

const INVALID_PACKAGE: &str = "built-in Grok provider package must be valid";

pub fn runtime() -> ProviderRuntime {
    ProviderRuntime::new(descriptor(), Arc::new(yolo_grok::GrokAdapter))
        .expect(INVALID_PACKAGE)
        .with_process_environment_keys(&[yolo_grok::grok::constants::DATA_HOME_ENV])
        .with_prompt_submit(PromptSubmit::Atomic)
        .with_interrupt_delivery(InterruptDelivery::Key(
            yolo_grok::grok::constants::PROMPT_INTERRUPT_KEY,
        ))
        .with_launch_command(yolo_grok::grok::constants::EXECUTABLE)
        .with_interaction_semantics(InteractionSemantics::Terminal)
        .with_hook_client(HookClient::Ingress)
        .with_hook_codec(yolo_grok_hooks::codec())
        .expect(INVALID_PACKAGE)
        .with_install_spec(install_spec())
        .expect(INVALID_PACKAGE)
        .with_conversation_source(Arc::new(GrokConversationSource))
        .expect(INVALID_PACKAGE)
}

fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor::new(
        identity::PROVIDER_ID.into(),
        identity::kind(),
        yolo_grok_hooks::PROVIDER.into(),
        CONVERSATION_TOKEN.into(),
        ProviderTier::L5,
        ProviderCapabilities::FULL,
        DISPLAY_NAME.into(),
        None,
        AuthorityKind::GenericHookGeneration,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .expect(INVALID_PACKAGE)
}
