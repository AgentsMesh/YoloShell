use super::*;
use crate::agent::{
    AgentAdapter, AgentBinding, AgentEvidence, AgentKind, AgentRegistry, DetectionContext,
    EvidenceDiscoveryPolicy, ResumePlan,
};
use anyhow::{Result, anyhow};
use std::error::Error;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const FAILING_SESSION_ID: &str = "failing-session";

struct FailingAdapter;

impl AgentAdapter for FailingAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn matches_process(&self, _: &ObservedProcess) -> bool {
        true
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn process_session_id(&self, _: &ObservedProcess) -> Option<String> {
        Some(FAILING_SESSION_ID.to_owned())
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Err(anyhow!("evidence unavailable"))
    }

    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<AgentEvidence>> {
        Ok(None)
    }

    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            args: Vec::new(),
            environment: Default::default(),
        }
    }

    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

fn detector(registry: AgentRegistry) -> AgentDetector {
    AgentDetector::new(
        DetectionContext::new(
            Uuid::new_v4(),
            "/tmp".into(),
            [(AgentKind::Codex, "/tmp".into())],
        ),
        registry,
        None,
    )
    .unwrap()
}

#[test]
fn scan_errors_distinguish_inventory_from_detection() {
    let mut detector =
        detector(AgentRegistry::from_adapters(vec![Box::new(FailingAdapter)]).unwrap());
    let process_error =
        detect_scan(&mut detector, Err(anyhow!("inventory unavailable"))).unwrap_err();
    assert_eq!(
        process_error.code(),
        crate::agent::constants::error::PROCESS_SCAN_FAILED
    );
    assert!(process_error.to_string().contains("process scan"));
    assert!(process_error.source().is_some());

    let processes = vec![ObservedProcess {
        pid: 2,
        start_time: 2,
        parent_pid: 1,
        process_group: 2,
        foreground_group: 2,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        argv: vec![crate::agent::codex::constants::EXECUTABLE.into()],
        environment: None,
        cwd: Some("/tmp".into()),
        evidence_paths: None,
    }];
    let detection_error = detect_scan(&mut detector, Ok(&processes)).unwrap_err();
    assert_eq!(
        detection_error.code(),
        crate::agent::constants::error::DETECTION_FAILED
    );
    assert!(detection_error.to_string().contains("agent detection"));
    assert!(detection_error.source().is_some());
}

#[test]
fn successful_empty_scan_is_unchanged() {
    let mut detector = detector(AgentRegistry::from_adapters(Vec::new()).unwrap());
    assert_eq!(
        detect_scan(&mut detector, Ok(&[])).unwrap(),
        DetectionResult::Unchanged
    );
}
