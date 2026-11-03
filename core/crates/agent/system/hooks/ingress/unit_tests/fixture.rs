use crate::admission::select;
use crate::contract::{HookProcessIdentity, HookScope};
use crate::parent::candidate_from;
use crate::profile::{HookAncestry, HookIngressProfile};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_adapter_contract::{
    AgentAdapter, DetectionContext, EvidenceDiscoveryPolicy, SessionGenerationPolicy,
};
use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, ObservedProcess, ResumePlan,
};
use yolo_process::ProcessIdentity;

pub(super) const HOOK: &str = "/opt/yolo/yolo";
pub(super) const BASH: &str = "/usr/bin/bash";
pub(super) const NODE: &str = "/usr/bin/node";

pub(super) struct NodeAgent;

impl AgentAdapter for NodeAgent {
    fn kind(&self) -> AgentKind {
        AgentKind::Other(yolo_gemini_identity::KIND.into())
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        process.executable == NODE
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        SessionGenerationPolicy::HookRequired
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<AgentEvidence>> {
        Ok(None)
    }

    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: "node".into(),
            args: Vec::new(),
            environment: BTreeMap::new(),
        }
    }

    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

pub(super) fn profile() -> HookIngressProfile {
    HookIngressProfile {
        provider: "test",
        socket_env: "TEST_SOCKET",
        outbox_env: "TEST_OUTBOX",
        max_ancestors: 8,
        ancestry: HookAncestry::DetachedSession,
        require_peer_environment: false,
    }
}

pub(super) fn hook_scope(agent: HookProcessIdentity) -> HookScope {
    HookScope {
        terminal_id: Uuid::from_u128(1),
        runtime_epoch: Uuid::from_u128(2),
        agent,
    }
}

pub(super) fn process(pid: u32, parent: u32, executable: &str) -> ObservedProcess {
    ObservedProcess {
        pid,
        start_time: u64::from(pid),
        parent_pid: parent,
        process_group: pid as i32,
        foreground_group: pid as i32,
        executable: executable.into(),
        argv: vec![executable.into()],
        environment: None,
        cwd: Some("/work".into()),
        evidence_paths: None,
    }
}

pub(super) fn agent_process(pid: u32, parent: u32, scope: HookScope) -> ObservedProcess {
    let mut process = process(pid, parent, NODE);
    process.environment = Some(BTreeMap::from([
        (
            yolo_agent_core::constants::YOLO_TERMINAL_ID_ENV.into(),
            scope.terminal_id.to_string(),
        ),
        (
            yolo_agent_core::constants::YOLO_RUNTIME_EPOCH_ENV.into(),
            scope.runtime_epoch.to_string(),
        ),
    ]));
    process
}

