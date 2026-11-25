use super::*;
use yolo_agent_provider_contract::{AuthorityKind, ProviderTier};

#[test]
fn built_in_runtime_descriptors_cover_every_registered_provider() {
    let runtime = builtin_provider_runtime();
    let descriptors: Vec<_> = runtime.descriptors().collect();
    assert_eq!(descriptors.len(), 5);
    let codex = runtime
        .by_install_token(yolo_codex_provider::INSTALL_TOKEN)
        .unwrap();
    assert_eq!(codex.kind(), AgentKind::Codex);
    assert_eq!(codex.descriptor().target_tier(), ProviderTier::L5);
    assert_eq!(codex.descriptor().guaranteed_floor(), ProviderTier::L5);
    assert_eq!(
        codex.descriptor().authority_kind(),
        AuthorityKind::CodexOccurrence
    );
    // Codex's TUI cancels on Esc; a process SIGINT terminates it.
    assert_eq!(
        codex.interrupt_delivery().key(),
        Some(yolo_codex::codex::constants::PROMPT_INTERRUPT_KEY)
    );
    let claude = runtime
        .by_install_token(yolo_claude_provider::INSTALL_TOKEN)
        .unwrap();
    assert_eq!(claude.kind(), AgentKind::ClaudeCode);
    assert_eq!(
        claude.descriptor().conversation_token(),
        yolo_claude_provider::CONVERSATION_TOKEN
    );
    assert_eq!(
        claude.descriptor().authority_kind(),
        AuthorityKind::ClaudeGeneration
    );
    let qwen = runtime
        .by_install_token(yolo_qwen_provider::INSTALL_TOKEN)
        .unwrap();
    assert_eq!(
        qwen.kind(),
        AgentKind::Other(yolo_qwen_provider::identity::KIND.into())
    );
    assert_eq!(qwen.descriptor().target_tier(), ProviderTier::L5);
    assert_eq!(qwen.descriptor().guaranteed_floor(), ProviderTier::L5);
    assert_eq!(
        qwen.descriptor().authority_kind(),
        AuthorityKind::GenericHookGeneration
    );
    for capability in [
        qwen.descriptor().capabilities().detect,
        qwen.descriptor().capabilities().resume_exact,
        qwen.descriptor().capabilities().lifecycle,
        qwen.descriptor().capabilities().conversation,
        qwen.descriptor().capabilities().submit,
        qwen.descriptor().capabilities().interrupt,
        qwen.descriptor().capabilities().recovery,
    ] {
        assert!(capability);
    }
    // Qwen's TUI reads Esc; SIGINT leaves bracketed paste off.
    assert_eq!(
        qwen.interrupt_delivery().key(),
        Some(yolo_qwen::qwen::constants::PROMPT_INTERRUPT_KEY)
    );
    let gemini = runtime
        .by_install_token(yolo_gemini_provider::INSTALL_TOKEN)
        .unwrap();
    assert_eq!(
        gemini.kind(),
        AgentKind::Other(yolo_gemini_provider::identity::KIND.into())
    );
    assert_eq!(gemini.descriptor().target_tier(), ProviderTier::L5);
    assert_eq!(gemini.descriptor().guaranteed_floor(), ProviderTier::L5);
    assert_eq!(
        gemini.descriptor().conversation_token(),
        yolo_gemini_provider::CONVERSATION_TOKEN
    );
    assert_eq!(
        gemini.descriptor().authority_kind(),
        AuthorityKind::GenericHookGeneration
    );
    for capability in [
        gemini.descriptor().capabilities().detect,
        gemini.descriptor().capabilities().resume_exact,
        gemini.descriptor().capabilities().lifecycle,
        gemini.descriptor().capabilities().conversation,
        gemini.descriptor().capabilities().submit,
        gemini.descriptor().capabilities().interrupt,
        gemini.descriptor().capabilities().recovery,
    ] {
        assert!(capability);
    }
    // Gemini converts an Enter that follows pasted text within 30 ms into a
    // newline, so its submit settles the paste before the submit key.
    assert_eq!(
        gemini.prompt_submit().enter_gap(),
        yolo_gemini::gemini::constants::PROMPT_SUBMIT_SETTLE
    );
    assert!(!gemini.prompt_submit().enter_gap().is_zero());
    // Gemini reads its cancel key from the terminal; SIGINT would terminate it.
    assert_eq!(
        gemini.interrupt_delivery().key(),
        Some(yolo_gemini::gemini::constants::PROMPT_INTERRUPT_KEY)
    );
    let grok = runtime
        .by_install_token(yolo_grok_provider::INSTALL_TOKEN)
        .unwrap();
    assert_eq!(
        grok.kind(),
        AgentKind::Other(yolo_grok_provider::identity::KIND.into())
    );
    assert_eq!(grok.descriptor().target_tier(), ProviderTier::L5);
    assert_eq!(
        grok.descriptor().conversation_token(),
        yolo_grok_provider::CONVERSATION_TOKEN
    );
    assert_eq!(
        grok.descriptor().authority_kind(),
        AuthorityKind::GenericHookGeneration
    );
    // Grok's TUI cancels on Ctrl+C and has no SIGINT handler; SIGINT would
    // terminate the CLI instead of cancelling the turn.
    assert_eq!(
        grok.interrupt_delivery().key(),
        Some(yolo_grok::grok::constants::PROMPT_INTERRUPT_KEY)
    );
    assert!(runtime.by_manifest_id("missing.provider").is_none());
}

#[test]
fn only_the_turn_proof_provider_exposes_a_turn_end_probe() {
    let runtime = builtin_provider_runtime();
    assert!(runtime.turn_end_probe(&AgentKind::Codex).is_some());
    assert!(runtime.turn_end_probe(&AgentKind::ClaudeCode).is_none());
    assert!(
        runtime
            .turn_end_probe(&AgentKind::Other(yolo_qwen_provider::identity::KIND.into()))
            .is_none()
    );
    assert!(
        runtime
            .turn_end_probe(&AgentKind::Other(
                yolo_gemini_provider::identity::KIND.into()
            ))
            .is_none()
    );
}

#[test]
fn provider_owned_process_environment_keys_are_derived_from_runtime() {
    let keys = builtin_agent_registry().process_environment_keys();
    for key in [
        yolo_claude_hook_contract::CLAUDE_HOOK_TOKEN_ENV,
        yolo_claude_hook_contract::CLAUDE_HOOK_INVOCATION_ENV,
        yolo_claude_hook_contract::CLAUDE_SHIM_ACTIVE_ENV,
        yolo_claude_hook_contract::CLAUDE_SETTINGS_IDENTITY_ENV,
        yolo_qwen::qwen::constants::DATA_HOME_ENV,
        yolo_qwen::qwen::constants::RUNTIME_HOME_ENV,
        yolo_gemini::gemini::constants::DATA_HOME_ENV,
    ] {
        assert!(keys.contains(&key), "missing provider key: {key}");
    }
    for key in yolo_codex_hook_contract::SENSITIVE_ENVIRONMENT_NAMES {
        assert!(keys.contains(key), "missing Codex key: {key}");
    }
}

#[test]
fn process_snapshot_keys_include_the_session_and_pty_contract() {
    let keys = builtin_agent_registry().process_environment_keys();
    for key in [
        yolo_agent_adapter_contract::constants::HOME_ENV,
        yolo_agent_adapter_contract::constants::YOLO_TERMINAL_ID_ENV,
        yolo_agent_adapter_contract::constants::YOLO_RUNTIME_EPOCH_ENV,
        crate::process::PTY_SESSION_CAPABILITY_ENV,
    ] {
        assert!(keys.contains(&key), "missing base key: {key}");
    }
}

#[test]
fn default_agent_registry_uses_the_builtin_runtime() {
    let registry = builtin_agent_registry();
    assert_eq!(
        registry
            .adapters()
            .map(|adapter| adapter.kind())
            .collect::<Vec<_>>(),
        vec![
            AgentKind::Codex,
            AgentKind::ClaudeCode,
            AgentKind::Other(yolo_qwen_provider::identity::KIND.into()),
            AgentKind::Other(yolo_gemini_provider::identity::KIND.into()),
            AgentKind::Other(yolo_grok_provider::identity::KIND.into()),
        ]
    );
}
