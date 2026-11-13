use super::constants::{OUTPUT_TEXT, RESPONSE_ITEM, SESSION_META};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_conversation_protocol::*;
use yolo_conversation_service::ConversationReader;
use yolo_hub_identity::*;

const NATIVE_SESSION_ID: &str = "01a1196f-84d9-7493-82bc-8468d635823f";
const BINDING_SCHEMA_VERSION: u32 = 5;

fn sources() -> yolo_conversation_service::ConversationSources {
    yolo_conversation_service::ConversationSources::new([
        std::sync::Arc::new(yolo_codex_provider::CodexConversationSource)
            as std::sync::Arc<dyn yolo_conversation_source::ConversationSource>,
        std::sync::Arc::new(yolo_claude_provider::ClaudeConversationSource),
        std::sync::Arc::new(yolo_qwen_provider::QwenConversationSource),
    ])
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub path: PathBuf,
    pub binding: AgentBinding,
    pub reader: ConversationReader,
    pub request: ConversationReadRequest,
}

impl Fixture {
    pub fn new(kind: AgentKind) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let cwd = root.join("project");
        let data_home = root.join("store");
        fs::create_dir(&cwd).unwrap();
        let path = match kind {
            AgentKind::Codex => data_home.join("sessions/2026/10/08").join(format!(
                "rollout-2026-10-08T00-00-00-{NATIVE_SESSION_ID}.jsonl"
            )),
            _ => data_home
                .join("projects/-project")
                .join(format!("{NATIVE_SESSION_ID}.jsonl")),
        };
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let header = match kind {
            AgentKind::Codex => json!({"type":SESSION_META,"payload":{
                "id":NATIVE_SESSION_ID,"cwd":cwd,"source":"cli"}}),
            _ => json!({"type":"progress","sessionId":NATIVE_SESSION_ID,"cwd":cwd}),
        };
        fs::write(&path, format!("{header}\n")).unwrap();
        let hub = HubId::new();
        let terminal = TerminalId::new();
        let binding = AgentBinding {
            schema_version: BINDING_SCHEMA_VERSION,
            terminal_id: terminal.as_uuid(),
            kind: kind.clone(),
            provider_session_id: NATIVE_SESSION_ID.into(),
            executable: kind.as_str().into(),
            cwd: cwd.to_str().unwrap().into(),
            evidence_path: path.to_str().unwrap().into(),
            data_home: data_home.to_str().unwrap().into(),
            process_pid: None,
            process_start_time: None,
            state: AgentState::WaitingForInput,
            recovery_availability: AgentRecoveryAvailability::Available,
            detected_at_ms: 1,
            updated_at_ms: 1,
        };
        let target = ConversationBinding {
            agent: AgentTarget {
                address: AgentAddress {
                    hub_id: hub,
                    agent_id: AgentId::new(),
                },
                execution_id: ExecutionId::new(),
            },
            runtime: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: hub,
                    terminal_id: terminal,
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
        };
        let reader = ConversationReader::new(sources());
        let descriptor = reader.resolve(&binding, target).unwrap();
        let request = ConversationReadRequest {
            address: descriptor.address,
            binding: target,
            cursor: None,
            limits: ConversationLimits::default(),
            since_revision: None,
            since_generation: None,
        };
        Self {
            directory,
            path,
            binding,
            reader,
            request,
        }
    }

    pub fn append(&self, value: Value) {
        self.append_bytes(format!("{value}\n").as_bytes());
    }

    pub fn append_bytes(&self, bytes: &[u8]) {
        fs::OpenOptions::new()
            .append(true)
            .open(&self.path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    pub fn message(&self, text: &str) {
        self.append(match self.binding.kind {
            AgentKind::Codex => json!({"type":RESPONSE_ITEM,"payload":{"type":"message",
                "role":"assistant","content":[{"type":OUTPUT_TEXT,"text":text}]}}),
            _ => json!({"type":"assistant","uuid":format!("message-{text}"),
                "message":{"content":[{"type":"text","text":text}]}}),
        });
    }

    pub fn page(&self) -> ConversationPage {
        match self.reader.read(&self.binding, &self.request).unwrap() {
            ConversationReadResult::Page { page } => page,
            _ => panic!("expected changed page"),
        }
    }
}
