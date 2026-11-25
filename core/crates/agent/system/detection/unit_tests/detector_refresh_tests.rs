use super::*;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

struct ReplacingAdapter {
    evidence: PathBuf,
}

impl AgentAdapter for ReplacingAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn matches_process(&self, _: &ObservedProcess) -> bool {
        true
    }

    fn owns_evidence_path(&self, path: &Path, _: &DetectionContext) -> bool {
        path == self.evidence
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(vec![self.evidence.clone()])
    }

    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        Ok(Some(AgentEvidence {
            provider_session_id: fs::read_to_string(path)?,
            cwd: cwd.into(),
            path: path.into(),
            data_home: path.parent().unwrap().into(),
            state: AgentState::Running,
        }))
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

#[test]
fn changed_evidence_path_is_reconsidered_for_a_later_process() {
    let directory = tempdir().unwrap();
    let evidence = directory.path().join("session.jsonl");
    let context = DetectionContext::new(
        Uuid::new_v4(),
        directory.path().into(),
        [(AgentKind::Codex, directory.path().into())],
    );
    let registry = AgentRegistry::from_adapters(vec![Box::new(ReplacingAdapter {
        evidence: evidence.clone(),
    })])
    .unwrap();
    let mut detector = AgentDetector::new(context, registry, None).unwrap();

    fs::write(&evidence, "first").unwrap();
    assert!(matches!(
        detector
            .detect(&[process(directory.path(), 10, &evidence)])
            .unwrap(),
        DetectionResult::BindingChanged(binding) if binding.provider_session_id == "first"
    ));

    fs::write(&evidence, "second").unwrap();
    assert!(matches!(
        detector
            .detect(&[process(directory.path(), 11, &evidence)])
            .unwrap(),
        DetectionResult::BindingChanged(binding) if binding.provider_session_id == "second"
    ));
}

fn process(cwd: &Path, pid: u32, evidence: &Path) -> ObservedProcess {
    ObservedProcess {
        pid,
        start_time: u64::from(pid),
        parent_pid: 1,
        process_group: 1,
        foreground_group: 1,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        argv: vec![crate::agent::codex::constants::EXECUTABLE.into()],
        environment: None,
        cwd: Some(cwd.into()),
        evidence_paths: Some(vec![evidence.into()]),
    }
}
