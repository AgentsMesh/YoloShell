use super::*;
use serde_json::json;
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_conversation_protocol::*;
use yolo_conversation_source::ConversationSources;
use yolo_hub_identity::*;

mod cache;
mod history;
mod prefix;
mod prefix_decisions;
mod registry;

pub(crate) fn sources() -> ConversationSources {
    ConversationSources::new([
        std::sync::Arc::new(yolo_codex_provider::CodexConversationSource)
            as std::sync::Arc<dyn yolo_conversation_source::ConversationSource>,
        std::sync::Arc::new(yolo_claude_provider::ClaudeConversationSource),
        std::sync::Arc::new(yolo_qwen_provider::QwenConversationSource),
        std::sync::Arc::new(yolo_gemini_provider::GeminiConversationSource),
        std::sync::Arc::new(yolo_grok_provider::GrokConversationSource),
    ])
}

pub(crate) fn binding() -> (AgentBinding, ConversationBinding) {
    let hub_id = HubId::new();
    let terminal_id = TerminalId::new();
    let target = ConversationBinding {
        agent: AgentTarget {
            address: AgentAddress {
                hub_id,
                agent_id: AgentId::new(),
            },
            execution_id: ExecutionId::new(),
        },
        runtime: RuntimeTarget {
            address: TerminalAddress {
                hub_id,
                terminal_id,
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
    };
    (
        AgentBinding {
            schema_version: 5,
            terminal_id: terminal_id.as_uuid(),
            kind: AgentKind::Codex,
            provider_session_id: "session".into(),
            executable: "codex".into(),
            cwd: "/project".into(),
            evidence_path: "/store/session".into(),
            data_home: "/store".into(),
            process_pid: None,
            process_start_time: None,
            state: AgentState::Unknown,
            recovery_availability: AgentRecoveryAvailability::Unavailable,
            detected_at_ms: 0,
            updated_at_ms: 0,
        },
        target,
    )
}

#[test]
fn conversation_identity_excludes_run_state_and_execution_but_includes_provider_store_and_session()
{
    let (mut binding, mut target) = binding();
    let initial = resolve_conversation(&binding, target, &sources()).unwrap();
    binding.state = AgentState::WaitingForPermission;
    binding.updated_at_ms += 1;
    target.agent.execution_id = ExecutionId::new();
    target.runtime.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        initial.address,
        resolve_conversation(&binding, target, &sources()).unwrap().address
    );
    binding.data_home = "/other".into();
    assert_ne!(
        initial.address,
        resolve_conversation(&binding, target, &sources()).unwrap().address
    );
    binding.kind = AgentKind::ClaudeCode;
    assert_eq!(
        resolve_conversation(&binding, target, &sources()).unwrap().provider,
        ConversationProvider::claude_code()
    );
    binding.kind = AgentKind::from_name(provider_tokens::QWEN);
    assert_eq!(
        resolve_conversation(&binding, target, &sources()).unwrap().provider,
        ConversationProvider::qwen_code()
    );
    binding.kind = AgentKind::Other("other".into());
    assert_eq!(
        resolve_conversation(&binding, target, &sources()),
        Err(ConversationError::UnsupportedProvider)
    );
}

#[test]
fn missing_and_mismatched_binding_fields_are_not_guessed() {
    let (binding, target) = binding();
    let mutations: [fn(&mut AgentBinding); 4] = [
        |binding| binding.provider_session_id.clear(),
        |binding| binding.evidence_path.clear(),
        |binding| binding.data_home.clear(),
        |binding| binding.terminal_id = uuid::Uuid::new_v4(),
    ];
    for mutate in mutations {
        let mut changed = binding.clone();
        mutate(&mut changed);
        assert_eq!(
            resolve_conversation(&changed, target, &sources()),
            Err(ConversationError::BindingChanged)
        );
    }
    let mut foreign = target;
    foreign.agent.address.hub_id = HubId::new();
    assert_eq!(
        resolve_conversation(&binding, foreign, &sources()),
        Err(ConversationError::BindingChanged)
    );
}

#[test]
fn invalid_requests_are_rejected_before_any_file_read() {
    let (binding, target) = binding();
    let descriptor = resolve_conversation(&binding, target, &sources()).unwrap();
    let reader = ConversationReader::new(sources());
    let request = ConversationReadRequest {
        address: descriptor.address,
        binding: target,
        cursor: None,
        limits: ConversationLimits {
            entries: 0,
            bytes: 1,
        },
        since_revision: None,
        since_generation: None,
    };
    assert_eq!(
        reader.read(&binding, &request),
        Err(ConversationError::InvalidRequest)
    );
}
