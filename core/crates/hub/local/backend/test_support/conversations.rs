use super::fixtures::constants::{NATIVE_OUTPUT_TEXT, NATIVE_RESPONSE_ITEM, NATIVE_SESSION_META};
use super::{cleanup_rpc::ProtocolFixture, connectors::get, native_support::*};
use serde_json::json;
use std::{fs, io::Write};
use yolo_conversation_protocol::*;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_runtime::model::TerminalView;

const SESSION: &str = "01a1196f-84d9-7493-82bc-8468d635823f";

pub struct Evidence {
    _root: tempfile::TempDir,
    pub view: TerminalView,
}

impl Evidence {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        let cwd = directory.join("project");
        let home = directory.join("store");
        fs::create_dir(&cwd).unwrap();
        let path = home
            .join("sessions/2026/10/08")
            .join(format!("rollout-2026-10-08T00-00-00-{SESSION}.jsonl"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let header =
            json!({"type":NATIVE_SESSION_META,"payload":{"id":SESSION,"cwd":cwd,"source":"cli"}});
        fs::write(&path, format!("{header}\n")).unwrap();
        let mut view = source();
        view.manifest.cwd = cwd.clone();
        let binding = view.agent.as_mut().unwrap();
        binding.provider_session_id = SESSION.into();
        binding.cwd = cwd.to_str().unwrap().into();
        binding.data_home = home.to_str().unwrap().into();
        binding.evidence_path = path.to_str().unwrap().into();
        Self { _root: root, view }
    }

    pub fn message(&self, text: &str) {
        let value = json!({"type":NATIVE_RESPONSE_ITEM,"payload":{"type":"message","role":"assistant",
            "content":[{"type":NATIVE_OUTPUT_TEXT,"text":text}]}});
        fs::OpenOptions::new()
            .append(true)
            .open(&self.view.agent.as_ref().unwrap().evidence_path)
            .unwrap()
            .write_all(format!("{value}\n").as_bytes())
            .unwrap();
    }
}

pub fn binding(snapshot: &AgentSnapshot) -> ConversationBinding {
    ConversationBinding {
        agent: snapshot.cursor().target(),
        runtime: snapshot.fields().terminal,
    }
}

pub fn request(descriptor: &ConversationDescriptor) -> ConversationReadRequest {
    ConversationReadRequest {
        address: descriptor.address.clone(),
        binding: descriptor.binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    }
}

pub async fn fence(fixture: &ProtocolFixture, view: &TerminalView) {
    let observed = observation(view, SOURCE_REVISION);
    fixture.reply(|request| get(request, view)).await;
    fixture
        .reply(|request| native(request, view, Some(&observed)))
        .await;
}

pub async fn two_fences(fixture: &ProtocolFixture, before: &TerminalView, after: &TerminalView) {
    fence(fixture, before).await;
    fence(fixture, after).await;
}
