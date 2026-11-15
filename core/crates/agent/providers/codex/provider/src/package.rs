//! Codex's compiled-in runtime row. The mechanism validates every identity
//! below; a mismatch fails at startup.
use crate::conversation::CodexConversationSource;
use crate::install::install_spec;
use crate::turn_end::CodexTurnEndProbe;
use crate::{CONVERSATION_TOKEN, DISPLAY_NAME, INSTALL_TOKEN, identity};
use std::sync::Arc;
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, InterruptDelivery, LifecycleSemantics, ProjectionPolicy, PromptSubmit,
    ProviderCapabilities, ProviderDescriptor, ProviderTier,
};
use yolo_agent_provider_runtime::{HookClient, ProviderRuntime};
use yolo_codex_hook_contract::SENSITIVE_ENVIRONMENT_NAMES;

const INVALID_PACKAGE: &str = "built-in Codex provider package must be valid";

pub fn runtime() -> ProviderRuntime {
    ProviderRuntime::new(descriptor(), Arc::new(yolo_codex::CodexAdapter))
        .expect(INVALID_PACKAGE)
        .with_process_environment_keys(SENSITIVE_ENVIRONMENT_NAMES)
        .with_prompt_submit(PromptSubmit::Atomic)
        .with_interrupt_delivery(InterruptDelivery::Key(
            yolo_codex::codex::constants::PROMPT_INTERRUPT_KEY,
        ))
        .with_launch_command(yolo_codex::codex::constants::EXECUTABLE)
        .with_interaction_semantics(InteractionSemantics::Terminal)
        .with_hook_client(HookClient::CodexNative)
        .with_turn_end_probe(Arc::new(CodexTurnEndProbe))
        .with_install_spec(install_spec())
        .expect(INVALID_PACKAGE)
        .with_conversation_source(Arc::new(CodexConversationSource))
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
        AuthorityKind::CodexOccurrence,
        LifecycleSemantics::StopCandidateWithTurnProof,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .expect(INVALID_PACKAGE)
}
