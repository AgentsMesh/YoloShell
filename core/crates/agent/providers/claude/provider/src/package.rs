//! Claude Code's compiled-in runtime row. The mechanism validates every
//! identity below; a mismatch fails at startup.
use crate::conversation::ClaudeConversationSource;
use crate::install::install_spec;
use crate::{CONVERSATION_TOKEN, DISPLAY_NAME, INSTALL_TOKEN, identity};
use std::sync::Arc;
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, InterruptDelivery, LifecycleSemantics, ProjectionPolicy, PromptSubmit,
    ProviderCapabilities, ProviderDescriptor, ProviderTier,
};
use yolo_agent_provider_runtime::{HookClient, ProviderRuntime};
use yolo_claude_hook_contract::{
    CLAUDE_HOOK_INVOCATION_ENV, CLAUDE_HOOK_TOKEN_ENV, CLAUDE_SETTINGS_IDENTITY_ENV,
    CLAUDE_SHIM_ACTIVE_ENV,
};

const INVALID_PACKAGE: &str = "built-in Claude provider package must be valid";
const ENVIRONMENT_KEYS: &[&str] = &[
    CLAUDE_HOOK_TOKEN_ENV,
    CLAUDE_HOOK_INVOCATION_ENV,
    CLAUDE_SHIM_ACTIVE_ENV,
    CLAUDE_SETTINGS_IDENTITY_ENV,
];

pub fn runtime() -> ProviderRuntime {
    ProviderRuntime::new(descriptor(), Arc::new(yolo_claude::ClaudeAdapter))
        .expect(INVALID_PACKAGE)
        .with_process_environment_keys(ENVIRONMENT_KEYS)
        .with_prompt_submit(PromptSubmit::Atomic)
        .with_interrupt_delivery(InterruptDelivery::Signal)
        .with_launch_command(yolo_claude::claude::constants::EXECUTABLE)
        .with_interaction_semantics(InteractionSemantics::Typed)
        .with_hook_client(HookClient::ClaudeNative)
        .with_install_spec(install_spec())
        .expect(INVALID_PACKAGE)
        .with_conversation_source(Arc::new(ClaudeConversationSource))
        .expect(INVALID_PACKAGE)
}

fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor::new(
        identity::PROVIDER_ID.into(),
        identity::kind(),
        INSTALL_TOKEN.into(),
        CONVERSATION_TOKEN.into(),
        ProviderTier::L5,
        ProviderCapabilities::FULL,
        DISPLAY_NAME.into(),
        None,
        AuthorityKind::ClaudeGeneration,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .expect(INVALID_PACKAGE)
}
