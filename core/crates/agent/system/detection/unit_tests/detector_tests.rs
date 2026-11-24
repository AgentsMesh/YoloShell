use super::scan::AgentScanError;
use super::*;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

use crate::agent::constants::MISSED_DETECTION_THRESHOLD;

const PROCESS_PID: u32 = 303;
const PROCESS_GROUP: i32 = 71;
const SESSION_ID: &str = "detected-session";
const FILESYSTEM_TIMESTAMP_TICK: Duration = Duration::from_millis(2);

struct CountingAdapter {
    path: PathBuf,
    scans: Arc<AtomicUsize>,
}

impl AgentAdapter for CountingAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        process.executable == crate::agent::codex::constants::EXECUTABLE
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn process_session_id(&self, _: &ObservedProcess) -> Option<String> {
        Some(SESSION_ID.to_owned())
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        self.scans.fetch_add(1, Ordering::SeqCst);
        Ok(vec![self.path.clone()])
    }

    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        let value: serde_json::Value = match fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            Some(value) => value,
            None => return Ok(None),
        };
        let Some(session_id) =
            value[crate::extensibility_constants::PROVIDER_SESSION_FIELD].as_str()
        else {
            return Ok(None);
        };
        Ok(Some(AgentEvidence {
            provider_session_id: session_id.into(),
            cwd: cwd.to_path_buf(),
            path: path.to_path_buf(),
            data_home: path.parent().unwrap().to_path_buf(),
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
fn strongly_scoped_detector_rescans_until_binding_then_uses_stable_evidence() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    fs::create_dir_all(&cwd).unwrap();
    let evidence = directory.path().join("evidence.jsonl");
    let scans = Arc::new(AtomicUsize::new(0));
    let registry = AgentRegistry::from_adapters(vec![Box::new(CountingAdapter {
        path: evidence.clone(),
        scans: Arc::clone(&scans),
    })])
    .unwrap();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(AgentKind::Codex, directory.path().into())],
    );
    let mut detector = AgentDetector::new(context, registry, None).unwrap();
    let foreground = process(&cwd, PROCESS_GROUP);
    let ordinary = ObservedProcess {
        executable: "printf".into(),
        ..foreground.clone()
    };
    assert_eq!(
        detector.detect(&[ordinary]).unwrap(),
        DetectionResult::Unchanged
    );
    let background = process(&cwd, PROCESS_GROUP + 1);
    assert_eq!(
        detector.detect(&[background]).unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(scans.load(Ordering::SeqCst), 0);

    fs::write(&evidence, b"{\"session_id\":").unwrap();
    assert_eq!(
        detector.detect(std::slice::from_ref(&foreground)).unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(scans.load(Ordering::SeqCst), 1);
    std::thread::sleep(FILESYSTEM_TIMESTAMP_TICK);
    fs::write(&evidence, format!("{{\"session_id\":\"{SESSION_ID}\"}}")).unwrap();
    assert!(matches!(
        detector.detect(std::slice::from_ref(&foreground)).unwrap(),
        DetectionResult::BindingChanged(binding) if binding.provider_session_id == SESSION_ID
    ));
    assert_eq!(
        detector.detect(&[foreground]).unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(scans.load(Ordering::SeqCst), 3);
}

#[test]
fn detector_requires_consecutive_misses_before_marking_a_binding_exited() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    fs::create_dir_all(&cwd).unwrap();
    let evidence = directory.path().join("evidence.jsonl");
    let registry = AgentRegistry::from_adapters(vec![Box::new(CountingAdapter {
        path: evidence.clone(),
        scans: Arc::new(AtomicUsize::new(0)),
    })])
    .unwrap();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(AgentKind::Codex, directory.path().into())],
    );
    let mut detector = AgentDetector::new(context, registry, None).unwrap();
    std::thread::sleep(FILESYSTEM_TIMESTAMP_TICK);
    fs::write(&evidence, format!("{{\"session_id\":\"{SESSION_ID}\"}}")).unwrap();
    assert!(matches!(
        detector.detect(&[process(&cwd, PROCESS_GROUP)]).unwrap(),
        DetectionResult::BindingChanged(_)
    ));
    for _ in 0..MISSED_DETECTION_THRESHOLD {
        assert!(matches!(
            detect_scan(&mut detector, Err(anyhow::anyhow!("ps failed"))),
            Err(AgentScanError::Process(_))
        ));
    }
    for _ in 1..MISSED_DETECTION_THRESHOLD {
        assert_eq!(detector.detect(&[]).unwrap(), DetectionResult::Unchanged);
    }
    assert!(matches!(
        detector.detect(&[]).unwrap(),
        DetectionResult::StatusChanged(binding)
            if binding.state == AgentState::Exited && binding.process_pid.is_none()
    ));
}

fn process(cwd: &Path, process_group: i32) -> ObservedProcess {
    ObservedProcess {
        pid: PROCESS_PID,
        start_time: u64::from(PROCESS_PID),
        parent_pid: 1,
        process_group,
        foreground_group: PROCESS_GROUP,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        argv: vec![crate::agent::codex::constants::EXECUTABLE.into()],
        environment: None,
        cwd: Some(cwd.to_path_buf()),
        evidence_paths: None,
    }
}
