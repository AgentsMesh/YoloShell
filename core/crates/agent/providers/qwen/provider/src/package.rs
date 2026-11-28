//! Qwen Code's compiled-in runtime row. The mechanism validates every
//! identity below; a mismatch fails at startup.
use crate::conversation::QwenConversationSource;
use crate::install::install_spec;
use crate::{CONVERSATION_TOKEN, DISPLAY_NAME, identity};
use std::sync::Arc;
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, InterruptDelivery, LifecycleSemantics, ProjectionPolicy, PromptSubmit,
    ProviderCapabilities, ProviderDescriptor, ProviderTier,
};
use yolo_agent_provider_runtime::{HookClient, ProviderRuntime};

const INVALID_PACKAGE: &str = "built-in Qwen provider package must be valid";
const ENVIRONMENT_KEYS: &[&str] = &[
    yolo_qwen::qwen::constants::DATA_HOME_ENV,
    yolo_qwen::qwen::constants::RUNTIME_HOME_ENV,
];

pub fn runtime() -> ProviderRuntime {
    ProviderRuntime::new(descriptor(), Arc::new(yolo_qwen::QwenAdapter))
        .expect(INVALID_PACKAGE)
        .with_process_environment_keys(ENVIRONMENT_KEYS)
        .with_prompt_submit(PromptSubmit::Atomic)
        .with_interrupt_delivery(InterruptDelivery::Key(
            yolo_qwen::qwen::constants::PROMPT_INTERRUPT_KEY,
        ))
        .with_launch_command(yolo_qwen::qwen::constants::EXECUTABLE)
        .with_interaction_semantics(InteractionSemantics::Terminal)
        .with_hook_client(HookClient::Ingress)
        .with_hook_codec(yolo_qwen_hooks::codec())
        .expect(INVALID_PACKAGE)
        .with_install_spec(install_spec())
        .expect(INVALID_PACKAGE)
        .with_conversation_source(Arc::new(QwenConversationSource))
        .expect(INVALID_PACKAGE)
}

fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor::new(
        identity::PROVIDER_ID.into(),
        identity::kind(),
        yolo_qwen_hooks::PROVIDER.into(),
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
