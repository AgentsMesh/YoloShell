use super::*;
use crate::agent::claude::generation::native::ClaudeNativeGenerationStore;
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::hooks::{Generation, ProcessIdentity};
use std::path::Path;

const CODEX_DATE: &str = "2026/09/09";
const CODEX_STAMP: &str = "2026-09-09T12-00-00";
const CLAUDE_PROJECT: &str = "native-project";
const INITIAL_SEQUENCE: u64 = 1;

pub(super) fn evidence(kind: &AgentKind, home: &Path, cwd: &Path, session: &str) -> PathBuf {
    let (path, value) = match kind {
        AgentKind::ClaudeCode => (
            home.join(crate::claude::constants::EVIDENCE_DIR)
                .join(CLAUDE_PROJECT)
                .join(session)
                .with_extension(crate::constants::JSONL_EXTENSION),
            serde_json::json!({ "type": "user", "sessionId": session, "cwd": cwd }),
        ),
        AgentKind::Other(kind) if kind == yolo_qwen_identity::KIND => (
            home.join("projects")
                .join(CLAUDE_PROJECT)
                .join("chats")
                .join(session)
                .with_extension(crate::constants::JSONL_EXTENSION),
            serde_json::json!({ "type": "user", "sessionId": session, "cwd": cwd }),
        ),
        _ => (
            home.join(crate::codex::constants::EVIDENCE_DIR)
                .join(CODEX_DATE)
                .join(format!(
                    "{}{CODEX_STAMP}-{session}.{}",
                    crate::codex::constants::ROLLOUT_FILE_PREFIX,
                    crate::constants::JSONL_EXTENSION
                )),
            serde_json::json!({
                (crate::codex::constants::RECORD_TYPE_KEY): crate::codex::constants::SESSION_META_TYPE,
                (crate::codex::constants::PAYLOAD_KEY): {
                    (crate::codex::constants::SESSION_ID_KEY): session,
                    (crate::codex::constants::CWD_KEY): cwd,
                    (crate::codex::constants::SOURCE_KEY): crate::codex::constants::CLI_SOURCE,
                }
            }),
        ),
    };
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut record = serde_json::to_vec(&value).unwrap();
    record.push(b'\n');
    std::fs::write(&path, record).unwrap();
    path
}

pub(super) fn publish(
    store: &AgentBindingStore,
    namespace: &NamespaceLease,
    runtime: AuthorityRuntimeIdentity,
    binding: &mut AgentBinding,
) -> PathBuf {
    if binding.kind == AgentKind::ClaudeCode {
        use crate::claude_hooks::*;
        let evidence = crate::claude::ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                Path::new(&binding.evidence_path),
                Path::new(&binding.cwd),
                Path::new(&binding.data_home),
                || {},
            )
            .unwrap()
            .unwrap();
        let store = ClaudeNativeGenerationStore::from_binding_path(store.path()).unwrap();
        let request = ClaudeNativeRequest {
            schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
            scope: ClaudeNativeScope {
                terminal_id: binding.terminal_id,
                runtime_epoch: runtime.runtime_epoch,
                agent: ClaudeProcessIdentity {
                    pid: binding.process_pid.unwrap(),
                    start_time: binding.process_start_time.unwrap(),
                },
            },
            event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
            event: ClaudeLifecycleEvent::SessionStart {
                event_id: Uuid::new_v4(),
                provider_session_id: binding.provider_session_id.clone(),
                source: ClaudeSessionStartSource::Fresh,
                cwd: binding.cwd.clone().into(),
                transcript_path: binding.evidence_path.clone().into(),
                state: ClaudeHookState::Running,
            },
        };
        let record = store
            .apply_in_namespace(
                namespace,
                &request,
                runtime.worker,
                &crate::process::boot_id(),
                &evidence,
                || Ok(()),
            )
            .unwrap();
        *binding = record.binding;
        store.path().to_owned()
    } else if matches!(&binding.kind, AgentKind::Other(kind) if kind == yolo_qwen_identity::KIND) {
        use yolo_agent_hook_generation::{
            EventKind, GenerationEvidence, GenerationRequest, GenerationScope,
            HookGenerationStore, HookMetadata, HookProviderKind, ProviderVersion,
            SessionStartSource, FIRST_SEQUENCE,
        };
        let evidence_path = Path::new(&binding.evidence_path).to_path_buf();
        let identity = crate::paths::identity::ObjectIdentity::from_file(
            &std::fs::File::open(&evidence_path).unwrap(),
        )
        .unwrap()
        .unwrap();
        let evidence = GenerationEvidence::new(
            binding.kind.clone(),
            binding.executable.clone(),
            binding.provider_session_id.clone(),
            Path::new(&binding.cwd).to_path_buf(),
            Path::new(&binding.data_home).to_path_buf(),
            evidence_path.clone(),
            Some(identity),
        )
        .unwrap();
        let request = GenerationRequest {
            scope: GenerationScope {
                terminal_id: binding.terminal_id,
                runtime_epoch: runtime.runtime_epoch,
                agent: AuthorityProcessIdentity {
                    pid: binding.process_pid.unwrap(),
                    start_time: binding.process_start_time.unwrap(),
                },
            },
            event_sequence: FIRST_SEQUENCE,
            event_id: Uuid::new_v4(),
            provider: HookProviderKind::Other(yolo_qwen_identity::KIND.into()),
            provider_version: ProviderVersion::new(0, 26, 2),
            provider_session_id: binding.provider_session_id.clone(),
            turn_id: None,
            agent_id: None,
            tool_name: None,
            tool_use_id: None,
            stop_hook_active: None,
            metadata: HookMetadata {
                cwd: Some(Path::new(&binding.cwd).to_path_buf()),
                transcript_path: Some(evidence_path),
            },
            event: EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        };
        let store = HookGenerationStore::from_binding_path(store.path()).unwrap();
        let record = store
            .apply_in_namespace(
                namespace,
                &request,
                runtime.worker,
                &crate::process::boot_id(),
                &evidence,
                || Ok(()),
            )
            .unwrap();
        store
            .publish_binding_in_namespace(namespace, &record, || Ok(()))
            .unwrap();
        *binding = record.binding;
        store.path().to_owned()
    } else {
        let authority = CodexBindingAuthority::new(
            runtime,
            ProcessIdentity::new(
                binding.process_pid.unwrap(),
                binding.process_start_time.unwrap(),
            ),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            CodexAuthorityVersion {
                route_revision: INITIAL_SEQUENCE,
                invocation_id: Uuid::new_v4(),
                generation: Generation::INITIAL,
                authority_revision: INITIAL_SEQUENCE,
                sequence: INITIAL_SEQUENCE,
            },
            CodexAuthorityStatus::Active,
            binding.clone(),
        )
        .unwrap()
        .with_verified_resume_identity(&crate::process::boot_id())
        .unwrap();
        let authority_store = store.codex_authority_store().unwrap();
        authority_store
            .publish_in_namespace_with_commit_fence(namespace, store.path(), &authority, || Ok(()))
            .unwrap();
        namespace.remove_file(store.path()).unwrap();
        authority_store.path().to_owned()
    }
}
